local Roact = require(script:FindFirstAncestor("Rojo").Roact)

local FitText = require(script.Parent.FitText)

local e = Roact.createElement

local ConnectionActivePanel = Roact.Component:extend("ConnectionActivePanel")

function ConnectionActivePanel:render()
	return e(FitText, {
		Kind = "TextLabel",
		Padding = Vector2.new(4, 4),
		Font = Enum.Font.SourceSans,
		TextSize = 16,
		Text = "Rojo Connected",
		TextColor3 = Color3.new(1, 1, 1),
		BackgroundColor3 = Color3.new(0, 0, 0),
		BorderSizePixel = 0,
		BackgroundTransparency = 0.6,
		Position = UDim2.new(0.5, 0, 0, 0),
		AnchorPoint = Vector2.new(0.5, 0),
	})
end

return ConnectionActivePanel