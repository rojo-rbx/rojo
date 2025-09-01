local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local Timer = require(Plugin.Timer)
local Assets = require(Plugin.Assets)
local Theme = require(Plugin.App.Theme)

local ScrollingFrame = require(Plugin.App.Components.ScrollingFrame)
local DisplayValue = require(Plugin.App.Components.PatchVisualizer.DisplayValue)

local e = Roact.createElement

local Dictionary = Roact.Component:extend("Dictionary")

function Dictionary:init()
	self:setState({
		diff = self:calculateDiff(),
	})
end

function Dictionary:calculateDiff()
	Timer.start("Dictionary:calculateDiff")
	local oldTable, newTable = self.props.oldTable or {}, self.props.newTable or {}

	-- Diff the two tables and find the added keys, removed keys, and changed keys
	local diff = {}

	for key, oldValue in oldTable do
		local newValue = newTable[key]
		if newValue == nil then
			table.insert(diff, {
				key = key,
				patchType = "Remove",
			})
		elseif newValue ~= oldValue then
			-- Note: should this do some sort of deep comparison for various types?
			table.insert(diff, {
				key = key,
				patchType = "Edit",
			})
		else
			table.insert(diff, {
				key = key,
				patchType = "Remain",
			})
		end
	end
	for key in newTable do
		if oldTable[key] == nil then
			table.insert(diff, {
				key = key,
				patchType = "Add",
			})
		end
	end

	table.sort(diff, function(a, b)
		return a.key < b.key
	end)

	Timer.stop()
	return diff
end

function Dictionary:didUpdate(previousProps)
	if previousProps.oldTable ~= self.props.oldTable or previousProps.newTable ~= self.props.newTable then
		self:setState({
			diff = self:calculateDiff(),
		})
	end
end

function Dictionary:render()
	local oldTable, newTable = self.props.oldTable or {}, self.props.newTable or {}
	local diff = self.state.diff

	return Theme.with(function(theme)
		local lines = table.create(#diff)
		for order, line in diff do
			local key = line.key
			local oldValue = oldTable[key]
			local newValue = newTable[key]

			table.insert(
				lines,
				e("Frame", {
					Size = UDim2.new(1, 0, 0, 25),
					LayoutOrder = order,
					BorderSizePixel = 0,
					BackgroundTransparency = if line.patchType == "Remain" then 1 else self.props.transparency,
					BackgroundColor3 = theme.Diff.Background[line.patchType],
				}, {
					DiffIcon = if line.patchType ~= "Remain"
						then e("ImageLabel", {
							Image = Assets.Images.Diff[line.patchType],
							ImageColor3 = theme.AddressEntry.PlaceholderColor,
							ImageTransparency = self.props.transparency,
							BackgroundTransparency = 1,
							Size = UDim2.new(0, 15, 0, 15),
							Position = UDim2.new(0, 7, 0.5, 0),
							AnchorPoint = Vector2.new(0, 0.5),
						})
						else nil,
					KeyName = e("TextLabel", {
						Size = UDim2.new(0.3, -15, 1, 0),
						Position = UDim2.new(0, 30, 0, 0),
						BackgroundTransparency = 1,
						Text = key,
						TextXAlignment = Enum.TextXAlignment.Left,
						FontFace = theme.Font.Main,
						TextSize = theme.TextSize.Body,
						TextColor3 = theme.Diff.Text[line.patchType],
						TextTruncate = Enum.TextTruncate.AtEnd,
					}),
					OldValue = e("Frame", {
						Size = UDim2.new(0.35, -7, 1, 0),
						Position = UDim2.new(0.3, 15, 0, 0),
						BackgroundTransparency = 1,
					}, {
						e(DisplayValue, {
							value = oldValue,
							transparency = self.props.transparency,
							textColor = theme.Diff.Text[line.patchType],
						}),
					}),
					NewValue = e("Frame", {
						Size = UDim2.new(0.35, -8, 1, 0),
						Position = UDim2.new(0.65, 8, 0, 0),
						BackgroundTransparency = 1,
					}, {
						e(DisplayValue, {
							value = newValue,
							transparency = self.props.transparency,
							textColor = theme.Diff.Text[line.patchType],
						}),
					}),
				})
			)
		end

		return Roact.createFragment({
			Headers = e("Frame", {
				Size = UDim2.new(1, 0, 0, 25),
				BackgroundTransparency = self.props.transparency:map(function(t)
					return 0.95 + (0.05 * t)
				end),
				BackgroundColor3 = theme.Diff.Row,
			}, {
				ColumnA = e("TextLabel", {
					Size = UDim2.new(0.3, -15, 1, 0),
					Position = UDim2.new(0, 30, 0, 0),
					BackgroundTransparency = 1,
					Text = "Key",
					TextXAlignment = Enum.TextXAlignment.Left,
					FontFace = theme.Font.Bold,
					TextSize = theme.TextSize.Body,
					TextColor3 = theme.Settings.Setting.DescriptionColor,
					TextTruncate = Enum.TextTruncate.AtEnd,
				}),
				ColumnB = e("TextLabel", {
					Size = UDim2.new(0.35, -7, 1, 0),
					Position = UDim2.new(0.3, 15, 0, 0),
					BackgroundTransparency = 1,
					Text = "Old",
					TextXAlignment = Enum.TextXAlignment.Left,
					FontFace = theme.Font.Bold,
					TextSize = theme.TextSize.Body,
					TextColor3 = theme.Settings.Setting.DescriptionColor,
					TextTruncate = Enum.TextTruncate.AtEnd,
				}),
				ColumnC = e("TextLabel", {
					Size = UDim2.new(0.35, -8, 1, 0),
					Position = UDim2.new(0.65, 8, 0, 0),
					BackgroundTransparency = 1,
					Text = "New",
					TextXAlignment = Enum.TextXAlignment.Left,
					FontFace = theme.Font.Bold,
					TextSize = theme.TextSize.Body,
					TextColor3 = theme.Settings.Setting.DescriptionColor,
					TextTruncate = Enum.TextTruncate.AtEnd,
				}),
				Separator = e("Frame", {
					Size = UDim2.new(1, 0, 0, 1),
					Position = UDim2.new(0, 0, 1, 0),
					BackgroundTransparency = 0,
					BorderSizePixel = 0,
					BackgroundColor3 = theme.BorderedContainer.BorderColor,
				}),
			}),
			KeyValues = e(ScrollingFrame, {
				position = UDim2.new(0, 1, 0, 25),
				size = UDim2.new(1, -2, 1, -27),
				scrollingDirection = Enum.ScrollingDirection.Y,
				transparency = self.props.transparency,
			}, {
				Layout = e("UIListLayout", {
					SortOrder = Enum.SortOrder.LayoutOrder,
					VerticalAlignment = Enum.VerticalAlignment.Top,
				}),
				Lines = Roact.createFragment(lines),
			}),
		})
	end)
end

return Dictionary
