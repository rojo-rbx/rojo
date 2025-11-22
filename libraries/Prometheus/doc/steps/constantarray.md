---
description: >-
  This Step will Extract all Constants and put them into an Array at the
  beginning of the script
---

# ConstantArray

### Settings

| Name                 | type    | description                                                                                                  |
| -------------------- | ------- | ------------------------------------------------------------------------------------------------------------ |
| Treshold             | number  | The relative amount of nodes that will be affected"                                                          |
| StringsOnly          | boolean | Wether to only Extract Strings                                                                               |
| Shuffle              | boolean | Wether to shuffle the order of Elements in the Array                                                         |
| Rotate               | boolean | Wether to rotate the String Array by a specific (random) amount. This will be undone on runtime.             |
| LocalWrapperTreshold | number  | The relative amount of nodes functions, that will get local wrappers                                         |
| LocalWrapperCount    | number  | The number of Local wrapper Functions per scope. This only applies if LocalWrapperTreshold is greater than 0 |
| LocalWrapperArgCount | number  | The number of Arguments to the Local wrapper Functions                                                       |
| MaxWrapperOffset     | number  | The Max Offset for the Wrapper Functions                                                                     |

### Example

{% code title="in.lua" %}
```lua
print("1")
print("2")
print("3")
print("4")
```
{% endcode %}

{% code title="out.lua" %}
```lua
-- LocalWrapperCount    = 3
-- LocalWrapperArgCount = 5
local F = {"4", "3", "2", "1"}
do
    local y, G = 1, 4
    while y < G do
        F[y], F[G] = F[G], F[y]
        y, G = y + 1, G - 1
    end
    y, G = 1, 3
    while y < G do
        F[y], F[G] = F[G], F[y]
        y, G = y + 1, G - 1
    end
    y, G = 4, 4
    while y < G do
        F[y], F[G] = F[G], F[y]
        y, G = y + 1, G - 1
    end
end
local function y(y)
    return F[y + 440]
end
local G = {cb = function(F, G, R, p, b)
        return y(G - 2277)
    end, n = function(F, G, R, p, b)
        return y(p + 47178)
    end, B = function(F, G, R, p, b)
        return y(F + 31775)
    end}
print(G.cb(1575, 1840, 2367, 1293, 1280))
print(G.B(-32213, -31781, -31538, -32780, -32728))
print(G.B(-32214, -33004, -31973, -32125, -31855))
print(G.B(-32211, -31884, -31217, -32222, -31210))

```
{% endcode %}
