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
    /// Thinking / reasoning tokens, billed at the output rate. They do not
    /// carry into the next turn's context window.
    pub thinking: u64,
    pub cost: f64,
}

/// A positional marker dropped into the conversation. `after` is the number
/// of turns that preceded it (so it sits between turn `after` and `after+1`);
/// `label` is optional free text shown alongside it.
#[derive(Clone)]
pub struct Marker {
    pub after: usize,
    pub label: String,
}

/// If `input` is a `marker` command, return its (possibly empty) label.
/// Accepts `marker`, `marker some text`, and `marker: some text`.
pub fn marker_label(input: &str) -> Option<String> {
    let t = input.trim();
    if t.eq_ignore_ascii_case("marker") {
        return Some(String::new());
    }
    let lower = t.to_lowercase();
    for prefix in ["marker:", "marker "] {
        if lower.starts_with(prefix) {
            return Some(t[prefix.len()..].trim().to_string());
        }
    }
    None
}

/// The raw line that persists a marker with the given label.
fn marker_raw(label: &str) -> String {
    if label.is_empty() {
        "marker".to_string()
    } else {
        format!("marker: {label}")
    }
}

/// Pull an inline marker out of a turn line. If any comma/semicolon-separated
/// segment is a `marker` / `marker: text` directive, return the line with that
/// segment removed together with the marker's label; otherwise return the line
/// (rejoined) and `None`. Only the first such segment is treated as a marker.
fn extract_inline_marker(line: &str) -> (String, Option<String>) {
    let mut label = None;
    let mut kept = Vec::new();
    for segment in line.split([',', ';']) {
        let seg = segment.trim();
        if seg.is_empty() {
            continue;
        }
        if label.is_none() {
            if let Some(l) = marker_label(seg) {
                label = Some(l);
                continue;
            }
        }
        kept.push(seg.to_string());
    }
    (kept.join(", "), label)
}

/// An open conversation tab. Holds everything that distinguishes one
/// conversation from another. The *active* tab's data lives in the App's
/// "live" fields (turns, input, …); this struct is where an inactive tab's
/// state is stashed, and is swapped back into the live fields on switch.
#[derive(Clone, Default)]
pub struct Tab {
    pub active_model: Option<usize>,
    pub turns: Vec<Turn>,
    pub carried_cached: u64,
    pub input: String,
    pub status: String,
    pub input_history: Vec<String>,
    pub history_pos: Option<usize>,
    pub scroll_up: u16,
    /// Positional markers the user dropped into this conversation.
    pub markers: Vec<Marker>,
    /// The saved-conversation name this tab was loaded from (or last saved as),
    /// used to default the "save as" prompt so re-saving overrides the entry.
    pub conv_name: Option<String>,
}

/// A one-line summary of a tab for the tab bar.
pub struct TabSummary {
    pub label: String,
    pub active: bool,
}

/// Which view is currently on screen.
#[derive(PartialEq, Clone, Copy)]
pub enum Screen {
    ModelSelect,
    CustomModel,
    Start,
    Chat,
    SaveName,
    /// Confirming deletion of the highlighted saved conversation.
    ConfirmDelete,
}

/// An entry on the "Start" picker: a blank conversation, a built-in example,
/// or a previously saved conversation.
#[derive(Clone)]
pub enum StartItem {
    Blank,
    Example { name: String, turns: Vec<String> },
    Saved { name: String, model: Model, turns: Vec<String>, path: std::path::PathBuf },
}

impl StartItem {
    pub fn label(&self) -> String {
        match self {
            StartItem::Blank => "Blank conversation".into(),
            StartItem::Example { name, turns } => {
                format!("Example: {name}  ({} turns)", turns.len())
            }
            StartItem::Saved { name, model, turns, .. } => {
                // Markers are saved as turn lines too; don't count them.
                let n = turns.iter().filter(|t| marker_label(t).is_none()).count();
                format!("Saved: {name}  — {} ({} turns)", model.name, n)
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

    // Lines the user actually submitted, in order, for Up/Down recall. Unlike
    // `turns`, this keeps the original typed line (e.g. a `repeat N` directive)
    // and stores one entry per submission rather than per expanded turn.
    pub input_history: Vec<String>,

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

    // How many lines the turn history is scrolled up from the bottom. 0 sticks
    // to the newest turn; mouse-wheel up increases it to reveal older turns.
    pub scroll_up: u16,

    // Positional markers in the active conversation (see `Tab::markers`).
    pub markers: Vec<Marker>,

    // Saved-conversation name backing the active tab (see `Tab::conv_name`).
    pub conv_name: Option<String>,

    // Max history scroll offset from the last render, so the scroll handlers
    // can clamp correctly against the real (marker-inclusive) line count.
    pub history_max_scroll: std::cell::Cell<u16>,

    // Open conversation tabs. The active tab's data lives in the fields above;
    // the others are stashed here. `active_tab` indexes into `tabs`.
    pub tabs: Vec<Tab>,
    pub active_tab: usize,

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
            input_history: Vec::new(),
            form: Default::default(),
            form_field: 0,
            start_items: Vec::new(),
            start_selected: 0,
            save_name: String::new(),
            history_pos: None,
            scroll_up: 0,
            markers: Vec::new(),
            conv_name: None,
            history_max_scroll: std::cell::Cell::new(0),
            tabs: vec![Tab::default()],
            active_tab: 0,
            should_quit: false,
        }
    }

    pub fn model(&self) -> Option<&Model> {
        self.active_model.and_then(|i| self.models.get(i))
    }

    pub fn total_cost(&self) -> f64 {
        self.turns.iter().map(|t| t.cost).sum()
    }

    pub fn total_tokens(&self) -> (u64, u64, u64, u64) {
        self.turns.iter().fold((0, 0, 0, 0), |(c, i, o, th), t| {
            (c + t.cached, i + t.input, o + t.output, th + t.thinking)
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
                path: c.path,
            });
        }
        self.start_items = items;
        self.start_selected = 0;
        self.screen = Screen::Start;
    }

    pub fn start_up(&mut self) {
        self.status.clear();
        if self.start_selected == 0 {
            self.start_selected = self.start_items.len().saturating_sub(1);
        } else {
            self.start_selected -= 1;
        }
    }

    pub fn start_down(&mut self) {
        self.status.clear();
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
            StartItem::Saved { name, model, turns, .. } => {
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
                // load_turns resets state, so record the source name afterwards
                // — it defaults the save prompt so re-saving overrides this file.
                self.conv_name = Some(name);
            }
        }
    }

    /// The highlighted start item's name if it is a deletable saved
    /// conversation, for the confirmation prompt.
    pub fn selected_saved_name(&self) -> Option<&str> {
        match self.start_items.get(self.start_selected) {
            Some(StartItem::Saved { name, .. }) => Some(name),
            _ => None,
        }
    }

    /// Ask to delete the highlighted start item: open the confirmation modal if
    /// it's a saved conversation, otherwise explain why it can't be deleted.
    pub fn request_delete(&mut self) {
        if self.selected_saved_name().is_some() {
            self.screen = Screen::ConfirmDelete;
        } else {
            self.status = "Only saved conversations can be deleted.".into();
        }
    }

    /// Confirm the pending deletion: remove the highlighted saved conversation
    /// from disk, rebuild the list (keeping the selection in range), and return
    /// to the Start picker.
    pub fn confirm_delete(&mut self) {
        let Some(StartItem::Saved { name, path, .. }) = self.start_items.get(self.start_selected)
        else {
            self.screen = Screen::Start;
            return;
        };
        let (name, path) = (name.clone(), path.clone());
        match storage::delete(&path) {
            Ok(()) => {
                let prev = self.start_selected;
                self.open_start();
                self.start_selected = prev.min(self.start_items.len().saturating_sub(1));
                self.status = format!("Deleted saved conversation '{name}'.");
            }
            Err(e) => {
                self.status = format!("Delete failed: {e}");
                self.screen = Screen::Start;
            }
        }
    }

    /// Dismiss the deletion modal without deleting.
    pub fn cancel_delete(&mut self) {
        self.screen = Screen::Start;
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
        self.input_history.clear();
        self.history_pos = None;
        self.scroll_up = 0;
        self.markers.clear();
        self.conv_name = None;
    }

    /// Apply a raw turn description: parse it, bill it, and append it.
    /// Returns false if the line had no recognizable token counts.
    fn apply_turn(&mut self, raw: String) -> bool {
        let raw = raw.trim().to_string();
        if raw.is_empty() {
            return false;
        }
        // A marker is a positional annotation, not a billed turn. Recording it
        // here lets a replayed (loaded) conversation restore its markers too.
        if let Some(label) = marker_label(&raw) {
            self.markers.push(Marker { after: self.turns.len(), label });
            return true;
        }
        // A marker may also ride inside a turn line (e.g. "300 prompt, 400 out,
        // marker: reviewed"). Strip it out before billing so the stored turn
        // stays a plain description, then drop the marker after the turn below.
        let (raw, inline_marker) = extract_inline_marker(&raw);
        let Some(model) = self.model().cloned() else {
            return false;
        };
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
            // Thinking tokens are billed like output, so they add to the cost
            // at the output rate.
            let cost = model.cost(cached, parsed.input, parsed.output + parsed.thinking);

            self.turns.push(Turn {
                raw: stored_raw.clone(),
                cached,
                input: parsed.input,
                output: parsed.output,
                thinking: parsed.thinking,
                cost,
            });

            // Everything this turn (prompt + tool inputs + outputs) plus what
            // was already cached now becomes cached input for the next turn.
            // Thinking tokens are excluded — they don't enter the context.
            self.carried_cached = cached + parsed.input + parsed.output;
        }

        // Drop the inline marker (if any) after the turn(s) just billed.
        if let Some(label) = inline_marker {
            self.markers.push(Marker { after: self.turns.len(), label });
        }
        true
    }

    /// Submit the current input buffer as a turn.
    pub fn submit_turn(&mut self) {
        let trimmed = self.input.trim();
        if trimmed.is_empty() {
            return;
        }
        // Record the line as typed for Up/Down recall, before it's parsed,
        // expanded, or cleared. Skip a run of identical lines so repeatedly
        // submitting the same turn doesn't clutter the recall list.
        let raw_input = trimmed.to_string();
        if self.input_history.last() != Some(&raw_input) {
            self.input_history.push(raw_input);
        }
        // `clear` wipes the current conversation.
        if trimmed.eq_ignore_ascii_case("clear") {
            self.reset_conversation();
            self.status = "Conversation cleared.".into();
            return;
        }
        // `marker` (optionally `marker: text`) drops a marker at the current
        // point in the conversation. It isn't a turn and isn't billed.
        if let Some(label) = marker_label(trimmed) {
            let after = self.turns.len();
            self.markers.push(Marker { after, label: label.clone() });
            self.input.clear();
            self.history_pos = None;
            self.status = if label.is_empty() {
                format!("Marker placed after turn {after}.")
            } else {
                format!("Marker '{label}' placed after turn {after}.")
            };
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
        // Jump back to the newest turn after sending.
        self.scroll_up = 0;
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
            // Make loaded lines recallable with Up/Down, skipping consecutive
            // duplicates as a live submission would.
            let trimmed = raw.trim().to_string();
            if !trimmed.is_empty() && self.input_history.last() != Some(&trimmed) {
                self.input_history.push(trimmed);
            }
            self.apply_turn(raw);
        }
        self.status = format!("Loaded {} turns. Continue typing to add more.", self.turns.len());
        self.screen = Screen::Chat;
    }

    /// The conversation as a flat list of raw lines for saving: each turn's
    /// raw text with `marker` lines woven in at their recorded positions. A
    /// marker placed after `k` turns lands between turn `k` and turn `k+1`.
    pub fn history_raws(&self) -> Vec<String> {
        let mut out = Vec::new();
        let emit_markers = |out: &mut Vec<String>, pos: usize| {
            for m in &self.markers {
                if m.after == pos {
                    out.push(marker_raw(&m.label));
                }
            }
        };
        for (i, t) in self.turns.iter().enumerate() {
            emit_markers(&mut out, i);
            out.push(t.raw.clone());
        }
        emit_markers(&mut out, self.turns.len());
        out
    }

    // --- Saving ----------------------------------------------------------

    /// Open the "save as" prompt (no-op if there's nothing to save).
    pub fn open_save(&mut self) {
        if self.turns.is_empty() {
            self.status = "Nothing to save yet.".into();
            return;
        }
        // Default to the name this conversation was loaded from (so pressing
        // Enter overrides that entry); otherwise suggest one from the model.
        self.save_name = self.conv_name.clone().unwrap_or_else(|| {
            self.model()
                .map(|m| format!("{} conversation", m.name))
                .unwrap_or_else(|| "conversation".into())
        });
        self.screen = Screen::SaveName;
    }

    /// Write the current conversation to disk under `save_name`.
    pub fn submit_save(&mut self) {
        let Some(model) = self.model().cloned() else {
            return;
        };
        let raws = self.history_raws();
        let name = self.save_name.trim().to_string();
        match storage::save(&name, &model, &raws) {
            Ok(path) => {
                // Remember the name so a later save defaults to overriding it.
                self.conv_name = Some(name);
                self.status = format!("Saved to {}", path.display());
            }
            Err(e) => {
                self.status = format!("Save failed: {e}");
            }
        }
        self.screen = Screen::Chat;
    }

    // --- Input history ---------------------------------------------------

    /// Recall an older submitted line into the input box (Up arrow). Cycles
    /// over the lines as typed, so a `repeat N` directive is preserved and a
    /// repeated turn is recalled once, not once per expanded turn.
    pub fn history_up(&mut self) {
        if self.input_history.is_empty() {
            return;
        }
        let pos = match self.history_pos {
            None => self.input_history.len() - 1,
            Some(0) => 0,
            Some(p) => p - 1,
        };
        self.history_pos = Some(pos);
        self.input = self.input_history[pos].clone();
    }

    /// Move toward more recent lines, clearing past the newest (Down arrow).
    pub fn history_down(&mut self) {
        match self.history_pos {
            Some(p) if p + 1 < self.input_history.len() => {
                self.history_pos = Some(p + 1);
                self.input = self.input_history[p + 1].clone();
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

    // --- Scrolling the turn history -------------------------------------

    /// Scroll the turn history up toward older turns (mouse wheel up).
    pub fn scroll_history_up(&mut self) {
        // Cap at the real scrollable distance from the last render, which
        // accounts for marker lines and any wrapping.
        let max = self.history_max_scroll.get();
        self.scroll_up = (self.scroll_up + 1).min(max);
    }

    /// Scroll the turn history down toward newer turns (mouse wheel down).
    pub fn scroll_history_down(&mut self) {
        self.scroll_up = self.scroll_up.saturating_sub(1);
    }

    // --- Conversation tabs ----------------------------------------------

    /// Copy the live conversation fields into the active tab's slot.
    fn stash_active(&mut self) {
        let t = &mut self.tabs[self.active_tab];
        t.active_model = self.active_model;
        t.turns = std::mem::take(&mut self.turns);
        t.carried_cached = self.carried_cached;
        t.input = std::mem::take(&mut self.input);
        t.status = std::mem::take(&mut self.status);
        t.input_history = std::mem::take(&mut self.input_history);
        t.history_pos = self.history_pos;
        t.scroll_up = self.scroll_up;
        t.markers = std::mem::take(&mut self.markers);
        t.conv_name = self.conv_name.take();
    }

    /// Load the active tab's stored state into the live conversation fields.
    fn restore_into_live(&mut self) {
        let t = self.tabs[self.active_tab].clone();
        self.active_model = t.active_model;
        self.turns = t.turns;
        self.carried_cached = t.carried_cached;
        self.input = t.input;
        self.status = t.status;
        self.input_history = t.input_history;
        self.history_pos = t.history_pos;
        self.scroll_up = t.scroll_up;
        self.markers = t.markers;
        self.conv_name = t.conv_name;
    }

    /// Switch the active tab to index `i`, stashing the current one first.
    fn switch_to(&mut self, i: usize) {
        if i == self.active_tab || i >= self.tabs.len() {
            return;
        }
        self.stash_active();
        self.active_tab = i;
        self.restore_into_live();
        self.screen = Screen::Chat;
    }

    /// Move to the next tab (wraps around). No-op with a single tab.
    pub fn next_tab(&mut self) {
        if self.tabs.len() <= 1 {
            return;
        }
        self.switch_to((self.active_tab + 1) % self.tabs.len());
    }

    /// Move to the previous tab (wraps around). No-op with a single tab.
    pub fn prev_tab(&mut self) {
        if self.tabs.len() <= 1 {
            return;
        }
        let i = (self.active_tab + self.tabs.len() - 1) % self.tabs.len();
        self.switch_to(i);
    }

    /// Open a fresh tab and route to model selection to set it up.
    pub fn new_tab(&mut self) {
        self.stash_active();
        self.tabs.push(Tab::default());
        self.active_tab = self.tabs.len() - 1;
        // Start the new tab from a clean slate.
        self.active_model = None;
        self.turns.clear();
        self.carried_cached = 0;
        self.input.clear();
        self.status.clear();
        self.history_pos = None;
        self.scroll_up = 0;
        self.markers.clear();
        self.conv_name = None;
        self.screen = Screen::ModelSelect;
    }

    /// Close the active tab and activate a neighbour. Keeps the last tab.
    pub fn close_tab(&mut self) {
        if self.tabs.len() <= 1 {
            self.status = "Can't close the last tab.".into();
            return;
        }
        self.tabs.remove(self.active_tab);
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }
        self.restore_into_live();
        self.screen = Screen::Chat;
    }

    /// The turns belonging to tab `i`. The active tab's data lives in the
    /// live `turns` field rather than its `tabs` slot, so it's special-cased.
    pub fn tab_turns(&self, i: usize) -> &[Turn] {
        if i == self.active_tab {
            &self.turns
        } else {
            &self.tabs[i].turns
        }
    }

    /// The model selected for tab `i` (active tab reads the live field).
    pub fn tab_model(&self, i: usize) -> Option<&Model> {
        let idx = if i == self.active_tab {
            self.active_model
        } else {
            self.tabs[i].active_model
        };
        idx.and_then(|m| self.models.get(m))
    }

    /// The largest cumulative (total) cost of any open tab. The cost charts
    /// use this to give every tab a shared y-scale, so switching tabs compares
    /// like with like instead of each tab rescaling to its own max.
    pub fn max_total_cost_across_tabs(&self) -> f64 {
        (0..self.tabs.len())
            .map(|i| self.tab_turns(i).iter().map(|t| t.cost).sum::<f64>())
            .fold(0.0_f64, f64::max)
    }

    /// The largest single-turn cost of any open tab, for a shared per-turn
    /// chart y-scale across tabs.
    pub fn max_turn_cost_across_tabs(&self) -> f64 {
        (0..self.tabs.len())
            .flat_map(|i| self.tab_turns(i).iter().map(|t| t.cost))
            .fold(0.0_f64, f64::max)
    }

    /// One-line summary per open tab, for the tab bar.
    pub fn tab_summaries(&self) -> Vec<TabSummary> {
        self.tabs
            .iter()
            .enumerate()
            .map(|(i, tab)| {
                let active = i == self.active_tab;
                // The active tab's data lives in the App fields, not in `tab`.
                let (turns, model_idx): (&Vec<Turn>, Option<usize>) = if active {
                    (&self.turns, self.active_model)
                } else {
                    (&tab.turns, tab.active_model)
                };
                let model_name = model_idx
                    .and_then(|m| self.models.get(m))
                    .map(|m| m.name.as_str())
                    .unwrap_or("—");
                let cost: f64 = turns.iter().map(|t| t.cost).sum();
                let label = format!("{}:{} ({}·${:.3})", i + 1, model_name, turns.len(), cost);
                TabSummary { label, active }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_inline_marker_pulls_label_and_cleans_line() {
        let (line, label) = extract_inline_marker("300 prompt, 400 out, marker: reviewed");
        assert_eq!(line, "300 prompt, 400 out");
        assert_eq!(label.as_deref(), Some("reviewed"));
    }

    #[test]
    fn extract_inline_marker_without_label() {
        let (line, label) = extract_inline_marker("300 prompt; 400 out; marker");
        assert_eq!(line, "300 prompt, 400 out");
        assert_eq!(label.as_deref(), Some(""));
    }

    #[test]
    fn extract_inline_marker_absent_leaves_line() {
        let (line, label) = extract_inline_marker("300 prompt, 400 out");
        assert_eq!(line, "300 prompt, 400 out");
        assert_eq!(label, None);
    }

    #[test]
    fn inline_marker_billed_turn_then_marker_after_it() {
        let mut app = App::new();
        app.active_model = Some(0);
        assert!(app.apply_turn("300 prompt, 400 out, marker: here".into()));
        // The turn is billed and stored without the marker text.
        assert_eq!(app.turns.len(), 1);
        assert_eq!(app.turns[0].raw, "300 prompt, 400 out");
        assert_eq!(app.turns[0].input, 300);
        assert_eq!(app.turns[0].output, 400);
        // The marker lands right after that turn.
        assert_eq!(app.markers.len(), 1);
        assert_eq!(app.markers[0].after, 1);
        assert_eq!(app.markers[0].label, "here");
    }
}
