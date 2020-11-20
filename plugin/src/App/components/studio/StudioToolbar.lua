local Rojo = script:FindFirstAncestor("Rojo")

local Roact = require(Rojo.Roact)

local StudioToolbarContext = require(script.Parent.StudioToolbarContext)
local StudioPluginContext = require(script.Parent.StudioPluginContext)

local e = Roact.createElement

local StudioToolbar = Roact.Component:extend("StudioToolbar")

function StudioToolbar:render()
	return e(StudioPluginContext.Consumer, {
		render = function(plugin)
			local name = self.props.name

			if not self.toolbar then
				self.toolbar = plugin:CreateToolbar(name)
			else
				self.toolbar.Name = name
			end

			return e(StudioToolbarContext.Provider, {
				value = self.toolbar,
			}, self.props[Roact.Children])
		end
	})
end

function StudioToolbar:willUnmount()
	self.toolbar:Destroy()
end

return StudioToolbar