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

local Array = Roact.Component:extend("Array")

function Array:init()
	self:setState({
		diff = self:calculateDiff(),
	})
end

function Array:calculateDiff()
	Timer.start("Array:calculateDiff")
	--[[
		Find the indexes that are added or removed from the array,
		and display them side by side with gaps for the indexes that
		dont exist in the opposite array.
	]]
	local oldTable, newTable = self.props.oldTable or {}, self.props.newTable or {}

	local i, j = 1, 1
	local diff = {}

	while i <= #oldTable and j <= #newTable do
		if oldTable[i] == newTable[j] then
			table.insert(diff, { oldTable[i], newTable[j] }) -- Unchanged
			i += 1
			j += 1
		elseif not table.find(newTable, oldTable[i], j) then
			table.insert(diff, { oldTable[i], nil }) -- Removal
			i += 1
		elseif not table.find(oldTable, newTable[j], i) then
			table.insert(diff, { nil, newTable[j] }) -- Addition
			j += 1
		else
			if table.find(newTable, oldTable[i], j) then
				table.insert(diff, { nil, newTable[j] }) -- Addition
				j += 1
			else
				table.insert(diff, { oldTable[i], nil }) -- Removal
				i += 1
			end
		end
	end

	-- Handle remaining elements
	while i <= #oldTable do
		table.insert(diff, { oldTable[i], nil }) -- Remaining Removals
		i += 1
	end
	while j <= #newTable do
		table.insert(diff, { nil, newTable[j] }) -- Remaining Additions
		j += 1
	end

	Timer.stop()
	return diff
end

function Array:didUpdate(previousProps)
	if previousProps.oldTable ~= self.props.oldTable or previousProps.newTable ~= self.props.newTable then
		self:setState({
			diff = self:calculateDiff(),
		})
	end
end

function Array:render()
	return Theme.with(function(theme)
		local diff = self.state.diff
		local lines = table.create(#diff)

		for i, element in diff do
			local oldValue = element[1]
			local newValue = element[2]

			local patchType = if oldValue == nil then "Add" elseif newValue == nil then "Remove" else "Remain"

			table.insert(
				lines,
				e("Frame", {
					Size = UDim2.new(1, 0, 0, 25),
					BackgroundTransparency = if patchType == "Remain" then 1 else self.props.transparency,
					BackgroundColor3 = theme.Diff.Background[patchType],
					BorderSizePixel = 0,
					LayoutOrder = i,
				}, {
					DiffIcon = if patchType ~= "Remain"
						then e("ImageLabel", {
							Image = Assets.Images.Diff[patchType],
							ImageColor3 = theme.AddressEntry.PlaceholderColor,
							ImageTransparency = self.props.transparency,
							BackgroundTransparency = 1,
							Size = UDim2.new(0, 15, 0, 15),
							Position = UDim2.new(0, 7, 0.5, 0),
							AnchorPoint = Vector2.new(0, 0.5),
						})
						else nil,
					Old = e("Frame", {
						Size = UDim2.new(0.5, -30, 1, 0),
						Position = UDim2.new(0, 30, 0, 0),
						BackgroundTransparency = 1,
					}, {
						Display = if oldValue ~= nil
							then e(DisplayValue, {
								value = oldValue,
								transparency = self.props.transparency,
								textColor = theme.Settings.Setting.DescriptionColor,
							})
							else nil,
					}),
					New = e("Frame", {
						Size = UDim2.new(0.5, -10, 1, 0),
						Position = UDim2.new(0.5, 5, 0, 0),
						BackgroundTransparency = 1,
					}, {
						Display = if newValue ~= nil
							then e(DisplayValue, {
								value = newValue,
								transparency = self.props.transparency,
								textColor = theme.Settings.Setting.DescriptionColor,
							})
							else nil,
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
					Size = UDim2.new(0.5, -30, 1, 0),
					Position = UDim2.new(0, 30, 0, 0),
					BackgroundTransparency = 1,
					Text = "Old",
					TextXAlignment = Enum.TextXAlignment.Left,
					FontFace = theme.Font.Bold,
					TextSize = theme.TextSize.Body,
					TextColor3 = theme.Settings.Setting.DescriptionColor,
					TextTruncate = Enum.TextTruncate.AtEnd,
				}),
				ColumnB = e("TextLabel", {
					Size = UDim2.new(0.5, -10, 1, 0),
					Position = UDim2.new(0.5, 5, 0, 0),
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

return Array
