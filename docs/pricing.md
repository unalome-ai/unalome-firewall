# Pricing Module

**Source file:** `src-tauri/src/parsers/pricing.rs`

## Purpose

None of the agents' local data files include dollar cost amounts. This module converts raw token counts into estimated USD costs based on known per-model pricing.

## Function

```rust
pub fn estimate_cost_usd(
    model: &str,         // Model name (substring-matched)
    input_tokens: u64,
    cache_write_tokens: u64,
    cache_read_tokens: u64,
    output_tokens: u64,
) -> f64
```

Returns the estimated cost in USD.

## Pricing table

Rates are per 1 million tokens. Model matching is by substring (e.g., `"claude-sonnet-4-6-20260318"` matches `"sonnet"`).

### Anthropic models

| Model contains | Input | Cache write | Cache read | Output |
|---|---|---|---|---|
| `opus` | $15.00 | $18.75 | $1.50 | $75.00 |
| `sonnet` | $3.00 | $3.75 | $0.30 | $15.00 |
| `haiku` | $0.80 | $1.00 | $0.08 | $4.00 |

### OpenAI models (for Cursor)

| Model contains | Input | Cache write | Cache read | Output |
|---|---|---|---|---|
| `gpt-4o` | $2.50 | — | — | $10.00 |
| `gpt-4` | $10.00 | — | — | $30.00 |
| `gpt-3.5` | $0.50 | — | — | $1.50 |

### Default

If no model name matches, sonnet-tier pricing ($3.00 / $3.75 / $0.30 / $15.00) is used as the fallback.

## Formula

```
cost = (input_tokens * input_rate
      + cache_write_tokens * cache_write_rate
      + cache_read_tokens * cache_read_rate
      + output_tokens * output_rate)
      / 1,000,000
```

## Which parsers use this

| Parser | Uses pricing? | Notes |
|---|---|---|
| Claude Code | Yes | Extracts `usage` from assistant messages, calls `estimate_cost_usd` with model name |
| Claude Desktop | No | Logs don't contain token counts |
| Cursor | No | Database doesn't contain token counts |

## Accuracy notes

- Prices are hardcoded as of March 2026 Anthropic/OpenAI pricing. They need manual updates when pricing changes.
- Cache token rates only apply to Anthropic models. OpenAI cache write/read rates are set to $0.
- The model name matching is order-dependent — `gpt-4o` is checked before `gpt-4` to avoid the more general match catching both.
