local Rojo = script:FindFirstAncestor("Rojo")

local Roact = require(Rojo.Roact)

local StudioToolbarContext = require(script.Parent.StudioToolbarContext)

local e = Roact.createElement

local StudioToggleButton = Roact.Component:extend("StudioToggleButton")

StudioToggleButton.defaultProps = {
	enabled = true,
	active = false,
}

function StudioToggleButton:render()
	return e(StudioToolbarContext.Consumer, {
		render = function(toolbar)
			if not self.button then
				self.button = toolbar:CreateButton(
					self.props.name,
					self.props.tooltip,
					self.props.icon,
					self.props.text
				)

				self.button.Click:Connect(function()
					if self.props.onClick then
						self.props.onClick()
					end
				end)

				self.button.ClickableWhenViewportHidden = true
			end

			self.button.Enabled = self.props.enabled
			self.button:SetActive(self.props.active)

			return nil
		end
	})
end

function StudioToggleButton:willUnmount()
	self.button:Destroy()
end

return StudioToggleButton