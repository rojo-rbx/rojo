---
description: This Step splits Strings to a specific or random length
---

# SplitStrings

### Settings

| Name                      | type   | description                                                                                                                                                                            | Values                      |
| ------------------------- | ------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------- |
| Treshold                  | number | The relative amount of nodes that will be affected                                                                                                                                     | 0 <= x <= 1                 |
| MinLength                 | number | The minimal length for the chunks in that the Strings are splitted                                                                                                                     | x > 0                       |
| MaxLength                 | number | The maximal length for the chunks in that the Strings are splitted                                                                                                                     | x >= MinLength              |
| ConcatenationType         | enum   | The Functions used for Concatenation. Note that when using custom, the String Array will also be Shuffled                                                                              | "strcat", "table", "custom" |
| CustomFunctionType        | enum   | <p>The Type of Function code injection This Option only applies when custom Concatenation is selected.<br>Note that when chosing inline, the code size may increase significantly!</p> | "global", "local", "inline" |
| CustomLocalFunctionsCount | number | The number of local functions per scope. This option only applies when CustomFunctionType = local                                                                                      | x > 0                       |

### Example

{% code title="in.lua" %}
```lua
print("Hello, World!")
```
{% endcode %}

{% code title="out.lua" %}
```lua
-- MinLength = 1
-- MaxLength = 1
local f = function(f)
    local k, C = f[#f], ""
    for j = 1, #k, 1 do
        C = C .. k[f[j]]
    end
    return C
end
print(f({13, 11, 4, 12, 1, 6, 8, 10, 9, 7, 3, 2, 5, {"o", "d", "l", "l", "!", ",", "r", " ", "o", "W", "e", "l", "H"}}))

```
{% endcode %}
