//! Pricing models and cost math.

/// A model with its per-million-token prices (in USD).
#[derive(Clone, Debug)]
pub struct Model {
    pub name: String,
    pub input_per_m: f64,
    pub output_per_m: f64,
    pub cached_per_m: f64,
    /// Maximum context window in tokens.
    pub context_window: u64,
}

impl Model {
    pub fn new(name: &str, input: f64, output: f64, cached: f64, context_window: u64) -> Self {
        Self {
            name: name.to_string(),
            input_per_m: input,
            output_per_m: output,
            cached_per_m: cached,
            context_window,
        }
    }

    /// Cost in USD for a single turn's token usage.
    pub fn cost(&self, cached: u64, input: u64, output: u64) -> f64 {
        let m = 1_000_000.0;
        (cached as f64 / m) * self.cached_per_m
            + (input as f64 / m) * self.input_per_m
            + (output as f64 / m) * self.output_per_m
    }
}

/// The set of common models shipped with the app.
pub fn default_models() -> Vec<Model> {
    let ctx = 1_000_000;
    vec![
        // name,                input, output, cached  (USD per 1M tokens),  context window
        Model::new("Opus 4.8", 5.0, 25.0, 0.5, ctx),
        Model::new("Sonnet 4.6", 3.0, 15.0, 0.3, ctx),
        Model::new("Haiku 4.5", 1.0, 5.0, 0.1, ctx),
        Model::new("GPT-5", 1.25, 10.0, 0.125, ctx),
        Model::new("Gemini 2.5 Pro", 1.25, 10.0, 0.31, ctx),
    ]
}
