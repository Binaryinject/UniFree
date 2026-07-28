---
name: mimo-usage
description: Check Xiaomi MIMO token plan usage and remaining quota. Use when asking about token usage, remaining quota, plan details, or MIMO platform consumption statistics.
---

# MIMO Token Usage Checker

Query Xiaomi MIMO token plan usage via the platform API.

## Prerequisites

- Python 3.7+
- Valid cookies from https://platform.xiaomimimo.com

## Quick Usage

Run the usage checker script:

```bash
python C:\GIT\Mimo-Usage-Monitor\scripts\mimo_usage.py
```

Options:
- `--json` — Output raw JSON for scripting
- `--threshold N` — Alert threshold percent (default: 80)

## Cookie Setup

Cookies are stored in `C:\GIT\Mimo-Usage-Monitor\.mimo_cookies`.

To update cookies:
1. Open https://platform.xiaomimimo.com in browser
2. Log in to your account
3. Open DevTools (F12) → Application → Cookies
4. Copy all cookies and update the `.mimo_cookies` file (single line, semicolon-separated)

## Output

The script shows:
- **Monthly usage** — Current month token consumption
- **Plan usage** — Total plan token consumption
- Progress bars with color-coded warnings (green/yellow/red)
