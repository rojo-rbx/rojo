local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local Dictionary = require(Plugin.Dictionary)

local StudioPluginContext = require(script.Parent.StudioPluginContext)

local e = Roact.createElement

local StudioPluginAction  = Roact.Component:extend("StudioPluginAction")

function StudioPluginAction:init()
	self.pluginAction = self.props.plugin:CreatePluginAction(
		self.props.name, self.props.title, self.props.description, self.props.icon, self.props.bindable
	)

	self.pluginAction.Triggered:Connect(self.props.onTriggered)
end

function StudioPluginAction:render()
	return nil
end

function StudioPluginAction:willUnmount()
	self.pluginAction:Destroy()
end

local function StudioPluginActionWrapper(props)
	return e(StudioPluginContext.Consumer, {
		render = function(plugin)
			return e(StudioPluginAction, Dictionary.merge(props, {
				plugin = plugin,
			}))
		end,
	})
end

return StudioPluginActionWrapper
