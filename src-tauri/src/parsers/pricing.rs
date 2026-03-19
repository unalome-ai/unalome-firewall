/// Token-to-USD cost estimation by model name.
/// Rates are per 1 million tokens.
/// Sources: https://devtk.ai/en/blog/ai-api-pricing-comparison-2026/
///          https://platform.claude.com/docs/en/docs/build-with-claude/prompt-caching
///          https://developers.openai.com/api/docs/pricing

/// Per-million-token pricing for a given model.
struct ModelPricing {
    input: f64,
    cache_write: f64,
    cache_read: f64,
    output: f64,
}

fn lookup_pricing(model: &str) -> ModelPricing {
    let m = model.to_lowercase();

    // ── Anthropic ───────────────────────────────────────────────
    // Claude Opus 4.1 / 4 (legacy)
    if m.contains("opus-4-1") || m.contains("opus-4-0") || m.contains("claude-4-opus") {
        return ModelPricing { input: 15.00, cache_write: 18.75, cache_read: 1.50, output: 75.00 };
    }
    // Claude Opus 4.5 / 4.6 (current)
    if m.contains("opus") {
        return ModelPricing { input: 5.00, cache_write: 6.25, cache_read: 0.50, output: 25.00 };
    }
    // Claude Sonnet 4 / 4.5 / 4.6
    if m.contains("sonnet") {
        return ModelPricing { input: 3.00, cache_write: 3.75, cache_read: 0.30, output: 15.00 };
    }
    // Claude Haiku 4.5
    if m.contains("haiku-4.5") || m.contains("haiku-4-5") {
        return ModelPricing { input: 1.00, cache_write: 1.25, cache_read: 0.10, output: 5.00 };
    }
    // Claude Haiku 3.5 (legacy)
    if m.contains("haiku") {
        return ModelPricing { input: 0.80, cache_write: 1.00, cache_read: 0.08, output: 4.00 };
    }

    // ── OpenAI ──────────────────────────────────────────────────
    // GPT-5.4 Pro
    if m.contains("gpt-5.4-pro") || m.contains("gpt-5-4-pro") {
        return ModelPricing { input: 30.00, cache_write: 0.0, cache_read: 3.00, output: 180.00 };
    }
    // GPT-5.4 Nano
    if m.contains("gpt-5.4-nano") || m.contains("gpt-5-4-nano") {
        return ModelPricing { input: 0.20, cache_write: 0.0, cache_read: 0.02, output: 1.25 };
    }
    // GPT-5.4 Mini
    if m.contains("gpt-5.4-mini") || m.contains("gpt-5-4-mini") {
        return ModelPricing { input: 0.75, cache_write: 0.0, cache_read: 0.075, output: 4.50 };
    }
    // GPT-5.4
    if m.contains("gpt-5.4") || m.contains("gpt-5-4") {
        return ModelPricing { input: 2.50, cache_write: 0.0, cache_read: 0.25, output: 15.00 };
    }
    // GPT-5.3 Codex
    if m.contains("codex") || m.contains("gpt-5.3") {
        return ModelPricing { input: 2.00, cache_write: 0.0, cache_read: 0.20, output: 10.00 };
    }
    // GPT-5
    if m.contains("gpt-5") {
        return ModelPricing { input: 1.25, cache_write: 0.0, cache_read: 0.125, output: 10.00 };
    }
    // GPT-4.1 Mini
    if m.contains("gpt-4.1-mini") || m.contains("gpt-4-1-mini") {
        return ModelPricing { input: 0.80, cache_write: 0.0, cache_read: 0.20, output: 3.20 };
    }
    // GPT-4.1
    if m.contains("gpt-4.1") || m.contains("gpt-4-1") {
        return ModelPricing { input: 2.00, cache_write: 0.0, cache_read: 0.50, output: 8.00 };
    }
    // GPT-4o Mini
    if m.contains("gpt-4o-mini") {
        return ModelPricing { input: 0.15, cache_write: 0.0, cache_read: 0.075, output: 0.60 };
    }
    // GPT-4o
    if m.contains("gpt-4o") {
        return ModelPricing { input: 2.50, cache_write: 0.0, cache_read: 1.25, output: 10.00 };
    }
    // GPT-4 Turbo
    if m.contains("gpt-4-turbo") || m.contains("gpt-4-1106") || m.contains("gpt-4-0125") {
        return ModelPricing { input: 10.00, cache_write: 0.0, cache_read: 0.0, output: 30.00 };
    }
    // GPT-4 (legacy)
    if m.contains("gpt-4") {
        return ModelPricing { input: 10.00, cache_write: 0.0, cache_read: 0.0, output: 30.00 };
    }
    // GPT-3.5
    if m.contains("gpt-3.5") {
        return ModelPricing { input: 0.50, cache_write: 0.0, cache_read: 0.0, output: 1.50 };
    }
    // o4-mini
    if m.contains("o4-mini") {
        return ModelPricing { input: 4.00, cache_write: 0.0, cache_read: 1.00, output: 16.00 };
    }
    // o3
    if m.contains("o3-mini") {
        return ModelPricing { input: 1.10, cache_write: 0.0, cache_read: 0.55, output: 4.40 };
    }
    if m.contains("o3") {
        return ModelPricing { input: 2.00, cache_write: 0.0, cache_read: 0.0, output: 8.00 };
    }
    // o1
    if m.contains("o1-mini") {
        return ModelPricing { input: 1.10, cache_write: 0.0, cache_read: 0.55, output: 4.40 };
    }
    if m.contains("o1") {
        return ModelPricing { input: 15.00, cache_write: 0.0, cache_read: 7.50, output: 60.00 };
    }

    // ── Google ──────────────────────────────────────────────────
    if m.contains("gemini-3.1-pro") || m.contains("gemini-3-1-pro") {
        return ModelPricing { input: 2.00, cache_write: 0.0, cache_read: 0.0, output: 12.00 };
    }
    if m.contains("gemini-2.5-pro") || m.contains("gemini-2-5-pro") {
        return ModelPricing { input: 1.25, cache_write: 0.0, cache_read: 0.0, output: 10.00 };
    }
    if m.contains("gemini-2.5-flash") || m.contains("gemini-2-5-flash") {
        return ModelPricing { input: 0.15, cache_write: 0.0, cache_read: 0.0, output: 0.60 };
    }
    if m.contains("gemini-2.0-flash") || m.contains("gemini-2-0-flash") {
        return ModelPricing { input: 0.10, cache_write: 0.0, cache_read: 0.0, output: 0.40 };
    }
    if m.contains("gemini-1.5-pro") || m.contains("gemini-1-5-pro") {
        return ModelPricing { input: 1.25, cache_write: 0.0, cache_read: 0.0, output: 5.00 };
    }
    if m.contains("gemini-1.5-flash") || m.contains("gemini-1-5-flash") {
        return ModelPricing { input: 0.075, cache_write: 0.0, cache_read: 0.0, output: 0.30 };
    }
    if m.contains("gemini") {
        return ModelPricing { input: 1.25, cache_write: 0.0, cache_read: 0.0, output: 10.00 };
    }

    // ── xAI ─────────────────────────────────────────────────────
    if m.contains("grok-3") {
        return ModelPricing { input: 3.00, cache_write: 0.0, cache_read: 0.0, output: 15.00 };
    }
    if m.contains("grok") {
        return ModelPricing { input: 3.00, cache_write: 0.0, cache_read: 0.0, output: 15.00 };
    }

    // ── DeepSeek ────────────────────────────────────────────────
    if m.contains("deepseek-r1") {
        return ModelPricing { input: 0.55, cache_write: 0.0, cache_read: 0.14, output: 2.19 };
    }
    if m.contains("deepseek-v3") {
        return ModelPricing { input: 0.27, cache_write: 0.0, cache_read: 0.07, output: 1.10 };
    }
    if m.contains("deepseek") {
        return ModelPricing { input: 0.27, cache_write: 0.0, cache_read: 0.07, output: 1.10 };
    }

    // ── Mistral ─────────────────────────────────────────────────
    if m.contains("mistral-large") {
        return ModelPricing { input: 2.00, cache_write: 0.0, cache_read: 0.0, output: 6.00 };
    }
    if m.contains("mistral-small") || m.contains("mistral-3") {
        return ModelPricing { input: 0.20, cache_write: 0.0, cache_read: 0.0, output: 0.60 };
    }
    if m.contains("codestral") {
        return ModelPricing { input: 0.30, cache_write: 0.0, cache_read: 0.0, output: 0.90 };
    }
    if m.contains("mistral") {
        return ModelPricing { input: 2.00, cache_write: 0.0, cache_read: 0.0, output: 6.00 };
    }

    // ── Meta Llama ──────────────────────────────────────────────
    if m.contains("llama-3.3") || m.contains("llama-3-3") {
        return ModelPricing { input: 0.88, cache_write: 0.0, cache_read: 0.0, output: 0.88 };
    }
    if m.contains("llama-3.1-405b") || m.contains("llama-3-1-405b") {
        return ModelPricing { input: 3.00, cache_write: 0.0, cache_read: 0.0, output: 3.00 };
    }
    if m.contains("llama-3.1-70b") || m.contains("llama-3-1-70b") {
        return ModelPricing { input: 0.88, cache_write: 0.0, cache_read: 0.0, output: 0.88 };
    }
    if m.contains("llama") {
        return ModelPricing { input: 0.88, cache_write: 0.0, cache_read: 0.0, output: 0.88 };
    }

    // ── Default: Sonnet-tier pricing ────────────────────────────
    ModelPricing { input: 3.00, cache_write: 3.75, cache_read: 0.30, output: 15.00 }
}

pub fn estimate_cost_usd(
    model: &str,
    input_tokens: u64,
    cache_write_tokens: u64,
    cache_read_tokens: u64,
    output_tokens: u64,
) -> f64 {
    let p = lookup_pricing(model);

    (input_tokens as f64 * p.input
        + cache_write_tokens as f64 * p.cache_write
        + cache_read_tokens as f64 * p.cache_read
        + output_tokens as f64 * p.output)
        / 1_000_000.0
}
