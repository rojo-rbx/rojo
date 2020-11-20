local Rojo = script:FindFirstAncestor("Rojo")

local Roact = require(Rojo.Roact)

local e = Roact.createElement

local NotConnectedPage = Roact.Component:extend("NotConnectedPage")

function NotConnectedPage:render()
	return e("TextLabel", {
		Text = "NotConnected",
		TextSize = 24,
		TextTransparency = self.props.transparency,
		Size = UDim2.new(1, 0, 1, 0),
		BackgroundTransparency = 1,
	})
end

return NotConnectedPage