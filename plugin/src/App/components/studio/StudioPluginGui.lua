local Rojo = script:FindFirstAncestor("Rojo")

local Roact = require(Rojo.Roact)

local StudioPluginContext = require(script.Parent.StudioPluginContext)

local e = Roact.createElement

local StudioPluginGui = Roact.PureComponent:extend("StudioPluginGui")

StudioPluginGui.defaultProps = {
	initDockState = Enum.InitialDockState.Right,
	active = false,
	overridePreviousState = false,
	floatingSize = Vector2.new(0, 0),
	minimumSize = Vector2.new(0, 0),
}

function StudioPluginGui:render()
	return e(StudioPluginContext.Consumer, {
		render = function(plugin)
			if not self.pluginGui then
				local floatingSize = self.props.floatingSize
				local minimumSize = self.props.minimumSize

				local dockWidgetPluginGuiInfo = DockWidgetPluginGuiInfo.new(
					self.props.initDockState,
					self.props.active,
					self.props.overridePreviousState,
					floatingSize.X, floatingSize.Y,
					minimumSize.X, minimumSize.Y
				)

				self.pluginGui = plugin:CreateDockWidgetPluginGui(self.props.id, dockWidgetPluginGuiInfo)

				if self.props.onInitialState then
					self.props.onInitialState(self.pluginGui.Enabled)
				end
			else
				-- Make sure the initial state is preserved until something changes
				self.pluginGui.Enabled = self.props.active
			end

			self.pluginGui.Name = self.props.id
			self.pluginGui.Title = self.props.title

			return e(Roact.Portal, {
				target = self.pluginGui,
			}, self.props[Roact.Children])
		end
	})
end

function StudioPluginGui:willUnmount()
	self.pluginGui:Destroy()
end

return StudioPluginGui