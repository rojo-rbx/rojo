local TextService = game:GetService("TextService")

local Roact = require(script:FindFirstAncestor("Rojo").Roact)

local Dictionary = require(script.Parent.Parent.Dictionary)

local e = Roact.createElement

local FitText = Roact.Component:extend("FitText")

function FitText:init()
	self.sizeBinding, self.setSize = Roact.createBinding(UDim2.new())
end

function FitText:render()
	local kind = self.props.Kind or "TextLabel"

	local containerProps = Dictionary.merge(self.props, {
		Kind = Dictionary.None,
		Padding = Dictionary.None,
		MinSize = Dictionary.None,
		Size = self.sizeBinding
	})

	return e(kind, containerProps)
end

function FitText:didMount()
	self:updateTextMeasurements()
end

function FitText:didUpdate()
	self:updateTextMeasurements()
end

function FitText:updateTextMeasurements()
	local minSize = self.props.MinSize or Vector2.new(0, 0)
	local padding = self.props.Padding or Vector2.new(0, 0)

	local text = self.props.Text or ""
	local font = self.props.Font or Enum.Font.Legacy
	local textSize = self.props.TextSize or 12

	local measuredText = TextService:GetTextSize(text, textSize, font, Vector2.new(9e6, 9e6))
	local totalSize = UDim2.new(
		0, math.max(minSize.X, padding.X * 2 + measuredText.X),
		0, math.max(minSize.Y, padding.Y * 2 + measuredText.Y))

	self.setSize(totalSize)
end

return FitText