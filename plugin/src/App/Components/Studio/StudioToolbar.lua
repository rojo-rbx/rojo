local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local Dictionary = require(Plugin.Dictionary)

local StudioToolbarContext = require(script.Parent.StudioToolbarContext)
local StudioPluginContext = require(script.Parent.StudioPluginContext)

local e = Roact.createElement

local StudioToolbar = Roact.Component:extend("StudioToolbar")

function StudioToolbar:init()
	self.toolbar = self.props.plugin:CreateToolbar(self.props.name)
end

function StudioToolbar:render()
	return e(StudioToolbarContext.Provider, {
		value = self.toolbar,
	}, self.props[Roact.Children])
end

function StudioToolbar:didUpdate(lastProps)
	if self.props.name ~= lastProps.name then
		self.toolbar.Name = self.props.name
	end
end

function StudioToolbar:willUnmount()
	self.toolbar:Destroy()
end

local function StudioToolbarWrapper(props)
	return e(StudioPluginContext.Consumer, {
		render = function(plugin)
			return e(StudioToolbar, Dictionary.merge(props, {
				plugin = plugin,
			}))
		end,
	})
end

return StudioToolbarWrapper
