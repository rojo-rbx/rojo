---
description: This Step wraps all locals into Proxy Objects
---

# ProxifyLocals

### Settings

| Name        | type | description                                 | values                                  |
| ----------- | ---- | ------------------------------------------- | --------------------------------------- |
| LiteralType | enum | The type of the randomly generated literals | "dictionary", "number", "string", "any" |

### Example

{% code title="in.lua" %}
```lua
local x = "Hello, World!"
print(x)
```
{% endcode %}

{% code title="out.lua" %}
```lua
-- LiteralType = "dictionary"
local n = setmetatable
local D =
    n(
    {Wz = function()
        end},
    {__div = function(R, n)
            R.Wz = n
        end, __concat = function(R, n)
            return R.Wz
        end}
)
local R =
    n(
    {Js = "Hello, World!"},
    {__add = function(R, n)
            R.Js = n
        end, __index = function(R, n)
            return rawget(R, "Js")
        end}
)
print(R.Muirgen)
```
{% endcode %}
