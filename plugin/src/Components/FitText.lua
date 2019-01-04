local TextService = game:GetService("TextService")

local Roact = require(script:FindFirstAncestor("Rojo").Roact)

local e = Roact.createElement

local None = newproxy(false)
local function merge(...)
	local output = {}

	for i = 1, select("#", ...) do
		local source = select(i, ...)

		if source ~= nil then
			for key, value in pairs(source) do
				if value == None then
					output[key] = nil
				else
					output[key] = value
				end
			end
		end
	end

	return output
end

local FitText = Roact.Component:extend("FitText")

function FitText:init()
	self.sizeBinding, self.setSize = Roact.createBinding(UDim2.new())
end

function FitText:render()
	local kind = self.props.Kind

	local containerProps = merge(self.props, {
		Kind = None,
		Padding = None,
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
	local padding = self.props.Padding or Vector2.new(0, 0)

	local text = self.props.Text or ""
	local font = self.props.Font or Enum.Font.Legacy
	local textSize = self.props.TextSize or 12

	local measuredText = TextService:GetTextSize(text, textSize, font, Vector2.new(9e6, 9e6))
	local totalSize = UDim2.new(
		0, padding.X * 2 + measuredText.X,
		0, padding.Y * 2 + measuredText.Y)

	self.setSize(totalSize)
end

return FitText