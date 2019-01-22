local Roact = require(script:FindFirstAncestor("Rojo").Roact)

local Plugin = script:FindFirstAncestor("Plugin")

local Theme = require(Plugin.Theme)
local Assets = require(Plugin.Assets)

local FitList = require(Plugin.Components.FitList)
local FitText = require(Plugin.Components.FitText)

local e = Roact.createElement

local RoundBox = Assets.Slices.RoundBox
local WhiteCross = Assets.Sprites.WhiteCross

local function ConnectionActivePanel(props)
	local stopSession = props.stopSession

	return e(FitList, {
		containerKind = "ImageLabel",
		containerProps = {
			Image = RoundBox.asset,
			ImageRectOffset = RoundBox.offset + Vector2.new(0, RoundBox.size.Y / 2),
			ImageRectSize = RoundBox.size * Vector2.new(1, 0.5),
			SliceCenter = Rect.new(4, 4, 4, 4),
			ScaleType = Enum.ScaleType.Slice,
			BackgroundTransparency = 1,
			Position = UDim2.new(0.5, 0, 0, 0),
			AnchorPoint = Vector2.new(0.5, 0),
		},
		layoutProps = {
			FillDirection = Enum.FillDirection.Horizontal,
			VerticalAlignment = Enum.VerticalAlignment.Center,
		},
	}, {
		Text = e(FitText, {
			Padding = Vector2.new(12, 6),
			Font = Theme.ButtonFont,
			TextSize = 18,
			Text = "Rojo Connected",
			TextColor3 = Theme.PrimaryColor,
			BackgroundTransparency = 1,
		}),

		CloseContainer = e("ImageButton", {
			Size = UDim2.new(0, 30, 0, 30),
			BackgroundTransparency = 1,

			[Roact.Event.Activated] = function()
				stopSession()
			end,
		}, {
			CloseImage = e("ImageLabel", {
				Size = UDim2.new(0, 16, 0, 16),
				Position = UDim2.new(0.5, 0, 0.5, 0),
				AnchorPoint = Vector2.new(0.5, 0.5),
				Image = WhiteCross.asset,
				ImageRectOffset = WhiteCross.offset,
				ImageRectSize = WhiteCross.size,
				ImageColor3 = Theme.PrimaryColor,
				BackgroundTransparency = 1,
			}),
		}),
	})
end

return ConnectionActivePanel