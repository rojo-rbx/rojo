local Rojo = script:FindFirstAncestor("Rojo")

local Roact = require(Rojo.Roact)

local e = Roact.createElement

local ConnectedPage = Roact.Component:extend("ConnectedPage")

function ConnectedPage:render()
	return e("TextLabel", {
		Text = "Connected",
		TextSize = 24,
		TextTransparency = self.props.transparency,
		Size = UDim2.new(1, 0, 1, 0),
		BackgroundTransparency = 1,
	})
end

return ConnectedPage