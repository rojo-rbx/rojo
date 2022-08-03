local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local Dictionary = require(Plugin.Dictionary)

local StudioToolbarContext = require(script.Parent.StudioToolbarContext)

local e = Roact.createElement

local StudioToggleButton = Roact.Component:extend("StudioToggleButton")

StudioToggleButton.defaultProps = {
	enabled = true,
	active = false,
}

function StudioToggleButton:init()
	local button = self.props.toolbar:CreateButton(
		self.props.name,
		self.props.tooltip,
		self.props.icon,
		self.props.text
	)

	button.Click:Connect(function()
		if self.props.onClick then
			self.props.onClick()
		end
	end)

	button.ClickableWhenViewportHidden = true

	self.button = button
end

function StudioToggleButton:render()
	return nil
end

function StudioToggleButton:didUpdate(lastProps)
	if self.props.enabled ~= lastProps.enabled then
		self.button.Enabled = self.props.enabled
	end

	if self.props.icon ~= lastProps.icon then
		self.button.Icon = self.props.icon
	end

	if self.props.active ~= lastProps.active then
		self.button:SetActive(self.props.active)
	end
end

function StudioToggleButton:willUnmount()
	self.button:Destroy()
end

local function StudioToggleButtonWrapper(props)
	return e(StudioToolbarContext.Consumer, {
		render = function(toolbar)
			return e(StudioToggleButton, Dictionary.merge(props, {
				toolbar = toolbar,
			}))
		end,
	})
end

return StudioToggleButtonWrapper
