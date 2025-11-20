---
description: This Step will encrypt all String constants in your code
---

# EncryptStrings

## Settings

None

## Example

{% code title="in.lua" %}
```lua
print("Hello, World!")
```
{% endcode %}

{% code title="out.lua" %}
```lua
-- Settings: None
local x, F
do
    local k = math.floor
    local I = math.random
    local Y = table.remove
    local i = string.char
    local K = 0
    local J = 2
    local Q = {}
    local W = {}
    local q = 0
    local R = {}
    for F = 1, 256, 1 do
        R[F] = F
    end
    repeat
        local F = I(1, #R)
        local x = Y(R, F)
        W[x] = i(x - 1)
    until #R == 0
    local j = {}
    local function B()
        if #j == 0 then
            K = (K * 173 + 8408159861491) % 35184372088832
            repeat
                J = (J * 160) % 257
            until J ~= 1
            local F = J % 32
            local x = (k(K / 2 ^ (13 - (J - F) / 32)) % 4294967296) / 2 ^ F
            local I = k((x % 1) * 4294967296) + k(x)
            local Y = I % 65536
            local i = (I - Y) / 65536
            local Q = Y % 256
            local W = (Y - Q) / 256
            local q = i % 256
            local R = (i - q) / 256
            j = {Q, W, q, R}
        end
        return table.remove(j)
    end
    local d = {}
    x = setmetatable({}, {__index = d, __metatable = nil})
    function F(x, k)
        local I = d
        if I[k] then
        else
            j = {}
            local F = W
            K = k % 35184372088832
            J = k % 255 + 2
            local Y = string.len(x)
            I[k] = ""
            local i = 198
            for Y = 1, Y, 1 do
                i = ((string.byte(x, Y) + B()) + i) % 256
                I[k] = I[k] .. F[i + 1]
            end
        end
        return k
    end
end
print(x[F("\219\018Q%~Y\225\128u\128\208&\155", 6909832146399)])

```
{% endcode %}
