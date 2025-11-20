---
description: This Step Wraps the Entire Script into a Function
---

# WrapInFunction

### Settings

| Name       | type   | description              |
| ---------- | ------ | ------------------------ |
| Iterations | number | The Number Of Iterations |

### Example

{% code title="in.lua" %}
```lua
print("Hello, World!")
```
{% endcode %}

{% code title="out.lua" %}
```lua
-- Iterations = 1
return (function()
    print("Hello, World!")
end)()

```
{% endcode %}
