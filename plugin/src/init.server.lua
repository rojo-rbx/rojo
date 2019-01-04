if not plugin then
	return
end

local Roact = require(script.Parent.Roact)

Roact.setGlobalConfig({
	elementTracing = true,
})

local App = require(script.Components.App)

local app = Roact.createElement(App, {
	plugin = plugin,
})

Roact.mount(app, game:GetService("CoreGui"), "Rojo UI")

-- TODO: Detect another instance of Rojo coming online and shut down this one.