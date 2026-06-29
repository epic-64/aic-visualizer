//! Application state and update logic.

use crate::model::{default_models, Model};
use crate::parser;
use crate::storage;

/// One simulated conversation turn, with the tokens billed and the cost.
#[derive(Clone)]
pub struct Turn {
    pub raw: String,
    pub cached: u64,
    pub input: u64,
    pub output: u64,
    pub cost: f64,
}

/// Which view is currently on screen.
#[derive(PartialEq, Clone, Copy)]
pub enum Screen {
    ModelSelect,
    CustomModel,
    Start,
    Chat,
    SaveName,
}

/// An entry on the "Start" picker: a blank conversation, a built-in example,
/// or a previously saved conversation.
#[derive(Clone)]
pub enum StartItem {
    Blank,
    Example { name: String, turns: Vec<String> },
    Saved { name: String, model: Model, turns: Vec<String> },
}

impl StartItem {
    pub fn label(&self) -> String {
        match self {
            StartItem::Blank => "Blank conversation".into(),
            StartItem::Example { name, turns } => {
                format!("Example: {name}  ({} turns)", turns.len())
            }
            StartItem::Saved { name, model, turns } => {
                format!("Saved: {name}  — {} ({} turns)", model.name, turns.len())
            }
        }
    }
}

/// Built-in example conversations the user can start from.
fn example_conversations() -> Vec<(String, Vec<String>)> {
    vec![
        (
            "Quick Q&A".into(),
            vec![
                "200 prompt, 350 response".into(),
                "150 prompt, 500 response".into(),
                "120 prompt, 300 response".into(),
            ],
        ),
        (
            "Coding session with tools".into(),
            vec![
                "400 prompt, 8000 tools, 1200 out".into(),
                "300 prompt, 15000 tools, 2500 out".into(),
                "250 prompt, 6000 tools, 1800 out".into(),
            ],
        ),
        (
            "Long agentic run".into(),
            vec![
                "500 prompt, 20000 tools, 3000 out".into(),
                "300 prompt, 30000 tools, 4000 out".into(),
                "300 prompt, 45000 tools, 5000 out".into(),
                "300 prompt, 60000 tools, 6000 out".into(),
            ],
        ),
    ]
}

/// Remove any `repeat N` / `N times` segments from a turn line, leaving a
/// plain single-turn description. Used so an expanded repeat is stored (and
/// later saved) as individual turns rather than re-multiplying on reload.
fn strip_repeat(raw: &str) -> String {
    raw.split([',', ';'])
        .map(str::trim)
        .filter(|seg| {
            let l = seg.to_lowercase();
            !seg.is_empty() && !l.contains("repeat") && !l.contains("times")
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Fields in the custom-model form, in tab order.
pub const FORM_FIELDS: [&str; 5] =
    ["Name", "Input $/M", "Output $/M", "Cached $/M", "Context (tok)"];

pub struct App {
    pub screen: Screen,
    pub models: Vec<Model>,
    pub selected: usize,
    pub active_model: Option<usize>,

    // Chat state.
    pub turns: Vec<Turn>,
    pub carried_cached: u64,
    pub input: String,
    pub status: String,

    // Custom-model form state.
    pub form: [String; 5],
    pub form_field: usize,

    // Start picker state.
    pub start_items: Vec<StartItem>,
    pub start_selected: usize,

    // Save-as state.
    pub save_name: String,

    // Input-history cursor (Up/Down to recall previous turns).
    pub history_pos: Option<usize>,

    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            screen: Screen::ModelSelect,
            models: default_models(),
            selected: 0,
            active_model: None,
            turns: Vec::new(),
            carried_cached: 0,
            input: String::new(),
            status: String::new(),
            form: Default::default(),
            form_field: 0,
            start_items: Vec::new(),
            start_selected: 0,
            save_name: String::new(),
            history_pos: None,
            should_quit: false,
        }
    }

    pub fn model(&self) -> Option<&Model> {
        self.active_model.and_then(|i| self.models.get(i))
    }

    pub fn total_cost(&self) -> f64 {
        self.turns.iter().map(|t| t.cost).sum()
    }

    pub fn total_tokens(&self) -> (u64, u64, u64) {
        self.turns.iter().fold((0, 0, 0), |(c, i, o), t| {
            (c + t.cached, i + t.input, o + t.output)
        })
    }

    /// Tokens currently occupying the context window — i.e. everything that
    /// carries into the next turn as cache.
    pub fn context_used(&self) -> u64 {
        self.carried_cached
    }

    /// The active model's maximum context window (0 if none selected).
    pub fn context_max(&self) -> u64 {
        self.model().map(|m| m.context_window).unwrap_or(0)
    }

    // --- Model selection -------------------------------------------------

    pub fn select_up(&mut self) {
        if self.selected == 0 {
            self.selected = self.models.len().saturating_sub(1);
        } else {
            self.selected -= 1;
        }
    }

    pub fn select_down(&mut self) {
        if self.selected + 1 >= self.models.len() {
            self.selected = 0;
        } else {
            self.selected += 1;
        }
    }

    /// Choose the highlighted model and open the Start picker.
    pub fn choose_model(&mut self) {
        self.active_model = Some(self.selected);
        self.open_start();
    }

    // --- Start picker ----------------------------------------------------

    /// Build the list of start options (blank + examples + saved files).
    pub fn open_start(&mut self) {
        let mut items = vec![StartItem::Blank];
        for (name, turns) in example_conversations() {
            items.push(StartItem::Example { name, turns });
        }
        for c in storage::list() {
            items.push(StartItem::Saved {
                name: c.name,
                model: c.model,
                turns: c.turns,
            });
        }
        self.start_items = items;
        self.start_selected = 0;
        self.screen = Screen::Start;
    }

    pub fn start_up(&mut self) {
        if self.start_selected == 0 {
            self.start_selected = self.start_items.len().saturating_sub(1);
        } else {
            self.start_selected -= 1;
        }
    }

    pub fn start_down(&mut self) {
        if self.start_selected + 1 >= self.start_items.len() {
            self.start_selected = 0;
        } else {
            self.start_selected += 1;
        }
    }

    /// Act on the highlighted start option.
    pub fn start_choose(&mut self) {
        let Some(item) = self.start_items.get(self.start_selected).cloned() else {
            return;
        };
        match item {
            StartItem::Blank => {
                self.reset_conversation();
                self.status = "New conversation. Describe a turn and press Enter.".into();
                self.screen = Screen::Chat;
            }
            StartItem::Example { turns, .. } => self.load_turns(turns),
            StartItem::Saved { model, turns, .. } => {
                // Restore (and if needed register) the saved conversation's model.
                let idx = self
                    .models
                    .iter()
                    .position(|m| m.name == model.name && m.input_per_m == model.input_per_m);
                let idx = idx.unwrap_or_else(|| {
                    self.models.push(model);
                    self.models.len() - 1
                });
                self.active_model = Some(idx);
                self.selected = idx;
                self.load_turns(turns);
            }
        }
    }

    // --- Custom model form ----------------------------------------------

    pub fn open_form(&mut self) {
        self.form = Default::default();
        self.form_field = 0;
        self.screen = Screen::CustomModel;
    }

    pub fn form_next(&mut self) {
        self.form_field = (self.form_field + 1) % FORM_FIELDS.len();
    }

    pub fn form_prev(&mut self) {
        self.form_field = (self.form_field + FORM_FIELDS.len() - 1) % FORM_FIELDS.len();
    }

    /// Validate and add the custom model. Returns an error message on failure.
    pub fn submit_form(&mut self) -> Result<(), String> {
        let name = self.form[0].trim();
        if name.is_empty() {
            return Err("Name cannot be empty".into());
        }
        let input = self.form[1].trim().parse::<f64>().map_err(|_| "Input price must be a number")?;
        let output = self.form[2].trim().parse::<f64>().map_err(|_| "Output price must be a number")?;
        let cached = self.form[3].trim().parse::<f64>().map_err(|_| "Cached price must be a number")?;
        // Context window defaults to 1M when left blank.
        let ctx_raw = self.form[4].trim();
        let context = if ctx_raw.is_empty() {
            1_000_000
        } else {
            ctx_raw.parse::<u64>().map_err(|_| "Context must be a whole number of tokens")?
        };
        self.models.push(Model::new(name, input, output, cached, context));
        self.selected = self.models.len() - 1;
        self.screen = Screen::ModelSelect;
        Ok(())
    }

    // --- Chat ------------------------------------------------------------

    /// Clear the conversation back to an empty state.
    fn reset_conversation(&mut self) {
        self.turns.clear();
        self.carried_cached = 0;
        self.input.clear();
        self.history_pos = None;
    }

    /// Apply a raw turn description: parse it, bill it, and append it.
    /// Returns false if the line had no recognizable token counts.
    fn apply_turn(&mut self, raw: String) -> bool {
        let Some(model) = self.model().cloned() else {
            return false;
        };
        let raw = raw.trim().to_string();
        if raw.is_empty() {
            return false;
        }
        let Some(parsed) = parser::parse(&raw) else {
            return false;
        };

        // Store each expanded turn as a single-turn line (without the `repeat`
        // directive) so saving/reloading doesn't re-multiply the repeats.
        let stored_raw = strip_repeat(&raw);

        // Apply the turn `repeat` times; each repetition re-caches the
        // previous one, so the cost grows as context accumulates.
        for i in 0..parsed.repeat.max(1) {
            // Cached tokens billed this turn: an explicit override (first
            // repetition only), or what the conversation has accumulated.
            let cached = match parsed.cached_override {
                Some(c) if i == 0 => c,
                _ => self.carried_cached,
            };
            let cost = model.cost(cached, parsed.input, parsed.output);

            self.turns.push(Turn {
                raw: stored_raw.clone(),
                cached,
                input: parsed.input,
                output: parsed.output,
                cost,
            });

            // Everything this turn (prompt + tool inputs + outputs) plus what
            // was already cached now becomes cached input for the next turn.
            self.carried_cached = cached + parsed.input + parsed.output;
        }
        true
    }

    /// Submit the current input buffer as a turn.
    pub fn submit_turn(&mut self) {
        let trimmed = self.input.trim();
        if trimmed.is_empty() {
            return;
        }
        // `clear` wipes the current conversation.
        if trimmed.eq_ignore_ascii_case("clear") {
            self.reset_conversation();
            self.status = "Conversation cleared.".into();
            return;
        }
        let before = self.turns.len();
        let raw = self.input.clone();
        if !self.apply_turn(raw) {
            self.status = "Couldn't find any token counts in that line.".into();
            return;
        }
        let added = self.turns.len() - before;
        self.input.clear();
        self.history_pos = None;
        if added > 1 {
            self.status = format!(
                "Added {added} turns. {} tokens now cached.",
                self.carried_cached
            );
        } else {
            let last = self.turns.last().unwrap();
            self.status = format!(
                "Turn cost ${:.4}. {} tokens now cached.",
                last.cost, self.carried_cached
            );
        }
    }

    /// Replay a list of raw turns into a fresh conversation, then open chat.
    pub fn load_turns(&mut self, raws: Vec<String>) {
        self.reset_conversation();
        for raw in raws {
            self.apply_turn(raw);
        }
        self.status = format!("Loaded {} turns. Continue typing to add more.", self.turns.len());
        self.screen = Screen::Chat;
    }

    // --- Saving ----------------------------------------------------------

    /// Open the "save as" prompt (no-op if there's nothing to save).
    pub fn open_save(&mut self) {
        if self.turns.is_empty() {
            self.status = "Nothing to save yet.".into();
            return;
        }
        self.save_name = self
            .model()
            .map(|m| format!("{} conversation", m.name))
            .unwrap_or_else(|| "conversation".into());
        self.screen = Screen::SaveName;
    }

    /// Write the current conversation to disk under `save_name`.
    pub fn submit_save(&mut self) {
        let Some(model) = self.model().cloned() else {
            return;
        };
        let raws: Vec<String> = self.turns.iter().map(|t| t.raw.clone()).collect();
        match storage::save(self.save_name.trim(), &model, &raws) {
            Ok(path) => {
                self.status = format!("Saved to {}", path.display());
            }
            Err(e) => {
                self.status = format!("Save failed: {e}");
            }
        }
        self.screen = Screen::Chat;
    }

    // --- Input history ---------------------------------------------------

    /// Recall an older turn into the input box (Up arrow).
    pub fn history_up(&mut self) {
        if self.turns.is_empty() {
            return;
        }
        let pos = match self.history_pos {
            None => self.turns.len() - 1,
            Some(0) => 0,
            Some(p) => p - 1,
        };
        self.history_pos = Some(pos);
        self.input = self.turns[pos].raw.clone();
    }

    /// Move toward more recent turns, clearing past the newest (Down arrow).
    pub fn history_down(&mut self) {
        match self.history_pos {
            Some(p) if p + 1 < self.turns.len() => {
                self.history_pos = Some(p + 1);
                self.input = self.turns[p + 1].raw.clone();
            }
            Some(_) => {
                self.history_pos = None;
                self.input.clear();
            }
            None => {}
        }
    }

    /// Called when the user edits the input directly — drop the history cursor.
    pub fn on_input_edit(&mut self) {
        self.history_pos = None;
    }
}
