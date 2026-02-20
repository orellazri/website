+++
title = "Writing a Neovim AI Code Completion Plugin From Scratch"
date = "2026-02-20"

[taxonomies]
tags=["coding", "ai"]
+++

I've used AI code completion (Copilot and Cursor) long enough that I miss it when it's not there. But I never loved the "trust me" part: a closed service, running somewhere else. I wanted a version I could run locally, point at a model on my own machine, and understand end-to-end.

So I built [Corridor](https://github.com/orellazri/corridor.nvim), a small inline code completion plugin for Neovim.

I wanted it small and boring:

- Ghost text suggestions that appear as you type, like Copilot
- Works with local models via [LM Studio](https://lmstudio.ai)
- Uses FIM (Fill-in-the-Middle) completions, which are purpose-built for code infilling
- Small codebase I can actually understand and maintain

Also, this was the first time I wrote a Neovim plugin from scratch. I'm happy with how it turned out, but I'm sure there are improvements to be made.

## What is Fill-in-the-Middle (FIM)?

Most language models generate left-to-right. Great for chat, awkward for code completion. In an editor, your cursor is usually _in the middle of something_: inside an argument list, between two lines, halfway through a conditional. If the model only sees what comes before the cursor, it's guessing blind about what you're trying to fit into.

FIM solves this by giving the model both sides of the cursor. The prompt is split into three parts:

- **Prefix**: Everything before the cursor
- **Suffix**: Everything after the cursor
- **Middle**: The gap the model needs to fill

The model is trained with special tokens that mark these regions. For example, with Qwen-based models:

```text
<|fim_prefix|>
def calculate_total(items):
    total = 0
    for item in items:
<|fim_suffix|>
    return total
<|fim_middle|>
```

The model sees the function signature, the loop setup, _and_ the return statement, then fills the hole with something like `total += item.price`. Without the suffix, it tends to wander: more lines than you want, or a different shape of function altogether.

Different model families use different FIM tokens. CodeLlama uses `<PRE>`, `<SUF>`, `<MID>`. DeepSeek uses `<｜fim▁begin｜>`, `<｜fim▁hole｜>`, `<｜fim▁end｜>`. Corridor ships with presets for all the major ones:

```lua
M.fim_presets = {
  starcoder = { prefix = "<fim_prefix>", suffix = "<fim_suffix>", middle = "<fim_middle>" },
  codellama = { prefix = "<PRE>", suffix = "<SUF>", middle = "<MID>" },
  deepseek  = { prefix = "<｜fim▁begin｜>", suffix = "<｜fim▁hole｜>", middle = "<｜fim▁end｜>" },
  qwen      = { prefix = "<|fim_prefix|>", suffix = "<|fim_suffix|>", middle = "<|fim_middle|>" },
  codestral = { prefix = "<|fim_prefix|>", suffix = "<|fim_suffix|>", middle = "<|fim_middle|>" },
}
```

## Corridor

The flow is: you type in insert mode, Neovim fires `CursorMovedI`, Corridor debounces that, grabs context around the cursor, sends a FIM request, and renders the result as ghost text. Tab accepts, Shift-Tab dismisses.

### Gathering Context

The context module extracts the prefix and suffix from the current buffer. The interesting bit is the asymmetric context window: when you limit the context size, Corridor allocates 70% of the budget to lines _before_ the cursor and 30% to lines _after_:

```lua
local before_limit = math.floor(max_lines * 0.7)
before_start = math.max(0, row - before_limit)
```

The code you just wrote tends to be a better predictor than whatever happens to be below the cursor. With a 50-line budget, 35 lines back and 15 forward is usually more useful than 25/25.

The context module also detects whether the cursor is mid-line (non-whitespace characters exist after the cursor). This matters later when deciding whether to show multi-line or single-line suggestions.

### Cancelling Stale Requests

When you type quickly you end up with a burst of in-flight requests. You only want the newest one; everything else is stale. The catch is that `plenary.curl` doesn't give you a clean way to cancel an HTTP request mid-flight.

The solution is a monotonically increasing request ID:

```lua
local current_request_id = 0

M.cancel = function()
  current_request_id = current_request_id + 1
end

M.fetch_suggestion = function(context, callback)
  M.cancel()  -- Invalidate any in-flight request
  local my_request_id = current_request_id

  curl.post(endpoint, {
    ...
    callback = function(res)
      -- If our ID doesn't match, a newer request was made -- bail
      if my_request_id ~= current_request_id then return end
      ...
    end,
  })
end
```

The response still arrives and gets parsed, but if a newer request was started in the meantime, the callback just drops it. I repeat that check a few times (after the response, after `vim.schedule`, and right before rendering) because editor state changes fast and stale suggestions are extremely noticeable.

### Rendering Ghost Text

Neovim's extmark API makes ghost text pretty clean. The first line is rendered as an overlay at the cursor, and extra lines are rendered as virtual lines below:

```lua
M.show = function(text)
  local lines = vim.split(text, "\n", { plain = true })

  local extmark_opts = {
    virt_text = { { lines[1], hl_group } },
    virt_text_pos = "overlay",
  }

  if #lines > 1 then
    local virt_lines = {}
    for i = 2, #lines do
      table.insert(virt_lines, { { lines[i], hl_group } })
    end
    extmark_opts.virt_lines = virt_lines
  end

  vim.api.nvim_buf_set_extmark(buf, ns_id, line, col, extmark_opts)
end
```

The suggestion is styled with a custom `CorridorSuggestion` highlight group that defaults to `Comment` (typically gray/dimmed), so it's visually distinct from real code.

### Debouncing

Debouncing is straightforward with `vim.uv.new_timer()`. Every keystroke resets the timer; the request only fires after you pause for 250ms (configurable):

```lua
timer:stop()
timer:start(config.get("debounce_ms"), 0, vim.schedule_wrap(function()
  if vim.api.nvim_get_mode().mode == "i" then
    M._request_suggestion()
  end
end))
```

The keymap setup has a nice trick: Tab accepts the suggestion when one is visible, but falls through to its default behavior (indentation, completion menu navigation) when there's no suggestion:

```lua
local function with_suggestion_or_fallback(key, action)
  return function()
    if ui.current_suggestion then
      action()
    else
      local termcodes = vim.api.nvim_replace_termcodes(key, true, true, true)
      vim.api.nvim_feedkeys(termcodes, "n", false)
    end
  end
end
```
