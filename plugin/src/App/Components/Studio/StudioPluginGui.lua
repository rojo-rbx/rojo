local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local Dictionary = require(Plugin.Dictionary)

local StudioPluginContext = require(script.Parent.StudioPluginContext)

local e = Roact.createElement

local StudioPluginGui = Roact.PureComponent:extend("StudioPluginGui")

StudioPluginGui.defaultProps = {
	initDockState = Enum.InitialDockState.Right,
	active = false,
	overridePreviousState = false,
	floatingSize = Vector2.new(0, 0),
	minimumSize = Vector2.new(0, 0),
	zIndexBehavior = Enum.ZIndexBehavior.Sibling,
}

function StudioPluginGui:init()
	local floatingSize = self.props.floatingSize
	local minimumSize = self.props.minimumSize

	local dockWidgetPluginGuiInfo = DockWidgetPluginGuiInfo.new(
		self.props.initDockState,
		self.props.active,
		self.props.overridePreviousState,
		floatingSize.X, floatingSize.Y,
		minimumSize.X, minimumSize.Y
	)

	local pluginGui = self.props.plugin:CreateDockWidgetPluginGui(self.props.id, dockWidgetPluginGuiInfo)

	pluginGui.Name = self.props.id
	pluginGui.Title = self.props.title
	pluginGui.ZIndexBehavior = self.props.zIndexBehavior

	if self.props.onInitialState then
		self.props.onInitialState(pluginGui.Enabled)
	end

	pluginGui:BindToClose(function()
		if self.props.onClose then
			self.props.onClose()
		else
			pluginGui.Enabled = false
		end
	end)

	self.pluginGui = pluginGui
end

function StudioPluginGui:render()
	return e(Roact.Portal, {
		target = self.pluginGui,
	}, self.props[Roact.Children])
end

function StudioPluginGui:didUpdate(lastProps)
	if self.props.active ~= lastProps.active then
		-- This is intentionally in didUpdate to make sure the initial active state
		-- (if the PluginGui is open initially) is preserved.
		self.pluginGui.Enabled = self.props.active
	end
end

function StudioPluginGui:willUnmount()
	self.pluginGui:Destroy()
end

local function StudioPluginGuiWrapper(props)
	return e(StudioPluginContext.Consumer, {
		render = function(plugin)
			return e(StudioPluginGui, Dictionary.merge(props, {
				plugin = plugin,
			}))
		end,
	})
end

return StudioPluginGuiWrapper
