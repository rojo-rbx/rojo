local Roact = require(script:FindFirstAncestor("Rojo").Roact)

local Assets = require(script.Parent.Parent.Assets)

local FitList = require(script.Parent.FitList)
local FitText = require(script.Parent.FitText)

local e = Roact.createElement

local GrayBox = Assets.Slices.GrayBox

local ConnectionActivePanel = Roact.Component:extend("ConnectionActivePanel")

function ConnectionActivePanel:render()
	return e(FitList, {
		containerKind = "ImageButton",
		containerProps = {
			Image = GrayBox.asset,
			ImageRectOffset = GrayBox.offset,
			ImageRectSize = GrayBox.size,
			SliceCenter = GrayBox.center,
			ScaleType = Enum.ScaleType.Slice,
			BackgroundTransparency = 1,
			Position = UDim2.new(0.5, 0, 0, 0),
			AnchorPoint = Vector2.new(0.5, 0),
		},
	}, {
		Text = e(FitText, {
			Padding = Vector2.new(12, 6),
			Font = Enum.Font.SourceSans,
			TextSize = 18,
			Text = "Rojo Connected",
			TextColor3 = Color3.new(0.05, 0.05, 0.05),
			BackgroundTransparency = 1,
		}),
	})
end

return ConnectionActivePanel