# The language-model seat: settings and spend

The settings file of the seat bridge (`baylee-seat join`), which says
which model plays and what it may spend, and the spend book that holds the
caps across games. An example file:

```json
{
  "default": "sonnet",
  "caps": {
    "day_usd": 10,
    "month_usd": 60,
    "day_tokens": 20000000,
    "month_tokens": 200000000
  },
  "profiles": {
    "sonnet": {
      "provider": "anthropic",
      "model": "claude-sonnet-5-5",
      "effort": "medium",
      "game_usd": 3,
      "think_secs": 60
    },
    "opus": {
      "provider": "anthropic",
      "model": "claude-opus-5-5",
      "effort": "high",
      "max_tokens": 24000,
      "game_usd": 6
    },
    "deepseek": {
      "provider": "openai",
      "model": "deepseek-chat",
      "base_url": "https://api.deepseek.com/v1",
      "key_env": "DEEPSEEK_API_KEY",
      "answer": "json",
      "game_tokens": 3000000
    }
  }
}
```
