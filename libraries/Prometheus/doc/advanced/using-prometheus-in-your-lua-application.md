# Using Prometheus in your Lua Application

Prometheus can also be used as a library for your custom Lua Applications instead of using its cli tool.&#x20;

In order to do that you'll first need to clone the github repo:

```batch
git clone "https://github.com/levno-710/Prometheus.git"
```

After that, you'll need to copy everything within the src folder to your project. Let's say you created a folder named `prometheus`, where all the Prometheus files are located. You can the use the following code to obfuscate a string:

{% code title="use_prometheus.lua" %}
```lua
local Prometheus = require("prometheus.prometheus")

-- If you don't want console output
Prometheus.Logger.logLevel = Prometheus.Logger.LogLevel.Error

-- Your code
local code = 'print("Hello, World!")'

-- Create a Pipeline using the Strong preset
local pipeline = Prometheus.Pipeline:fromConfig(Prometheus.Presets.Strong)

-- Apply the obfuscation and print the result
print(pipeline:apply(code));
```
{% endcode %}

Instead of passing the Strong preset you could also pass a custom [Config Object](../getting-started/the-config-object.md).
