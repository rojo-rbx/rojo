local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)
local Log = require(Packages.Log)
local Highlighter = require(Packages.Highlighter)
Highlighter.matchStudioSettings()
local StringDiff = require(script:FindFirstChild("StringDiff"))

local Timer = require(Plugin.Timer)
local Assets = require(Plugin.Assets)
local Theme = require(Plugin.App.Theme)
local getTextBoundsAsync = require(Plugin.App.getTextBoundsAsync)

local BorderedContainer = require(Plugin.App.Components.BorderedContainer)
local VirtualScroller = require(Plugin.App.Components.VirtualScroller)

local e = Roact.createElement

local StringDiffVisualizer = Roact.Component:extend("StringDiffVisualizer")

function StringDiffVisualizer:init()
	self.scriptBackground, self.setScriptBackground = Roact.createBinding(Color3.fromRGB(0, 0, 0))
	self.updateEvent = Instance.new("BindableEvent")
	self.lineHeight, self.setLineHeight = Roact.createBinding(15)
	self.canvasPosition, self.setCanvasPosition = Roact.createBinding(Vector2.zero)
	self.windowWidth, self.setWindowWidth = Roact.createBinding(math.huge)

	-- Ensure that the script background is up to date with the current theme
	self.themeChangedConnection = settings().Studio.ThemeChanged:Connect(function()
		task.delay(1 / 20, function()
			-- Delay to allow Highlighter to process the theme change first
			self:updateScriptBackground()
			-- Refresh the code label colors too
			self.updateEvent:Fire()
		end)
	end)

	self:updateScriptBackground()

	self:setState({
		oldDiffs = {},
		newDiffs = {},
		oldSpacers = {},
		newSpacers = {},
	})
end

function StringDiffVisualizer:willUnmount()
	self.themeChangedConnection:Disconnect()
	self.updateEvent:Destroy()
end

function StringDiffVisualizer:updateScriptBackground()
	local backgroundColor = Highlighter.getTokenColor("background")
	if backgroundColor ~= self.scriptBackground:getValue() then
		self.setScriptBackground(backgroundColor)
	end
end

function StringDiffVisualizer:didUpdate(previousProps)
	if previousProps.oldString ~= self.props.oldString or previousProps.newString ~= self.props.newString then
		self:updateDiffs()
	end
end

function StringDiffVisualizer:calculateContentSize(theme)
	local oldString, newString = self.props.oldString, self.props.newString

	local oldStringBounds = getTextBoundsAsync(oldString, theme.Font.Code, theme.TextSize.Code, math.huge)
	local newStringBounds = getTextBoundsAsync(newString, theme.Font.Code, theme.TextSize.Code, math.huge)

	return Vector2.new(math.max(oldStringBounds.X, newStringBounds.X), math.max(oldStringBounds.Y, newStringBounds.Y))
end

function StringDiffVisualizer:updateDiffs()
	Timer.start("StringDiffVisualizer:updateDiffs")
	local oldString, newString = self.props.oldString, self.props.newString

	-- Diff the two texts
	local startClock = os.clock()
	local diffs = StringDiff.findDiffs((string.gsub(oldString, "\t", "    ")), (string.gsub(newString, "\t", "    ")))
	local stopClock = os.clock()

	Log.trace(
		"Diffing {} byte and {} byte strings took {} microseconds and found {} diff sections",
		#oldString,
		#newString,
		math.round((stopClock - startClock) * 1000 * 1000),
		#diffs
	)

	-- Find the diff locations
	local oldDiffs, newDiffs = {}, {}
	local oldSpacers, newSpacers = {}, {}

	local firstDiffLineNum = 0

	local oldLineNum, oldIdx, newLineNum, newIdx = 1, 0, 1, 0
	for _, diff in diffs do
		local actionType, text = diff.actionType, diff.value
		local lines = string.split(text, "\n")

		if actionType == StringDiff.ActionTypes.Equal then
			for i, line in lines do
				if i > 1 then
					oldLineNum += 1
					oldIdx = 0
					newLineNum += 1
					newIdx = 0
				end
				oldIdx += #line
				newIdx += #line
			end
		elseif actionType == StringDiff.ActionTypes.Insert then
			if firstDiffLineNum == 0 then
				firstDiffLineNum = newLineNum
			end

			for i, line in lines do
				if i > 1 then
					newLineNum += 1
					newIdx = 0

					table.insert(oldSpacers, { oldLineNum = oldLineNum, newLineNum = newLineNum })
				end
				if not newDiffs[newLineNum] then
					newDiffs[newLineNum] = {
						{ start = newIdx, stop = newIdx + #line },
					}
				else
					table.insert(newDiffs[newLineNum], {
						start = newIdx,
						stop = newIdx + #line,
					})
				end
				newIdx += #line
			end
		elseif actionType == StringDiff.ActionTypes.Delete then
			if firstDiffLineNum == 0 then
				firstDiffLineNum = oldLineNum
			end

			for i, line in lines do
				if i > 1 then
					oldLineNum += 1
					oldIdx = 0

					table.insert(newSpacers, { oldLineNum = oldLineNum, newLineNum = newLineNum })
				end
				if not oldDiffs[oldLineNum] then
					oldDiffs[oldLineNum] = {
						{ start = oldIdx, stop = oldIdx + #line },
					}
				else
					table.insert(oldDiffs[oldLineNum], {
						start = oldIdx,
						stop = oldIdx + #line,
					})
				end
				oldIdx += #line
			end
		else
			Log.warn("Unknown diff action: {} {}", actionType, text)
		end
	end

	-- Filter out diffs that are just newlines being added/removed from existing non-empty lines.
	-- This is done to make the diff visualization less noisy.

	local oldStringLines = string.split(oldString, "\n")
	local newStringLines = string.split(newString, "\n")

	for lineNum, lineDiffs in oldDiffs do
		if
			(#lineDiffs > 1) -- Not just newline
			or (lineDiffs[1].start ~= lineDiffs[1].stop) -- Not a newline at all
			or (oldStringLines[lineNum] == "") -- Empty line, so the newline change is significant
		then
			continue
		end
		-- Just a noisy newline diff, clear it
		oldDiffs[lineNum] = nil
	end

	for lineNum, lineDiffs in newDiffs do
		if
			(#lineDiffs > 1) -- Not just newline
			or (lineDiffs[1].start ~= lineDiffs[1].stop) -- Not a newline at all
			or (newStringLines[lineNum] == "") -- Empty line, so the newline change is significant
		then
			continue
		end
		-- Just a noisy newline diff, clear it
		newDiffs[lineNum] = nil
	end

	Timer.stop()

	self:setState({
		oldDiffs = oldDiffs,
		newDiffs = newDiffs,
		oldSpacers = oldSpacers,
		newSpacers = newSpacers,
	})
	-- Scroll to the first diff line
	self.setCanvasPosition(Vector2.new(0, math.max(0, (firstDiffLineNum - 4) * 16)))
end

function StringDiffVisualizer:render()
	local oldString, newString = self.props.oldString, self.props.newString
	local oldDiffs, newDiffs = self.state.oldDiffs, self.state.newDiffs
	local oldSpacers, newSpacers = self.state.oldSpacers, self.state.newSpacers

	return Theme.with(function(theme)
		self.setLineHeight(theme.TextSize.Code)

		local richTextLinesOldString = Highlighter.buildRichTextLines({
			src = oldString,
		})
		local richTextLinesNewString = Highlighter.buildRichTextLines({
			src = newString,
		})

		local maxLines = math.max(#richTextLinesOldString, #richTextLinesNewString)

		-- Calculate the width of the canvas
		-- (One line at a time to avoid the 200k char limit of getTextBoundsAsync)
		local canvasWidth = 0
		for i = 1, maxLines do
			local oldLine = richTextLinesOldString[i]
			if oldLine and oldLine ~= "" then
				local bounds = getTextBoundsAsync(oldLine, theme.Font.Code, theme.TextSize.Code, math.huge, true)
				if bounds.X > canvasWidth then
					canvasWidth = bounds.X
				end
			end
			local newLine = richTextLinesNewString[i]
			if newLine and oldLine ~= "" then
				local bounds = getTextBoundsAsync(newLine, theme.Font.Code, theme.TextSize.Code, math.huge, true)
				if bounds.X > canvasWidth then
					canvasWidth = bounds.X
				end
			end
		end

		-- Adjust the rich text lines and their diffs to include spacers (aka nil lines)
		for spacerIdx, spacer in oldSpacers do
			local spacerLineNum = spacer.oldLineNum + (spacerIdx - 1)
			table.insert(richTextLinesOldString, spacerLineNum, nil)
			-- The oldDiffs that come after this spacer need to be moved down
			-- without overwriting the oldDiffs that are already there
			local updatedOldDiffs = {}
			for lineNum, diffs in pairs(oldDiffs) do
				if lineNum >= spacerLineNum then
					updatedOldDiffs[lineNum + 1] = diffs
				else
					updatedOldDiffs[lineNum] = diffs
				end
			end
			oldDiffs = updatedOldDiffs
		end
		for spacerIdx, spacer in newSpacers do
			local spacerLineNum = spacer.newLineNum + (spacerIdx - 1)
			table.insert(richTextLinesNewString, spacerLineNum, nil)
			-- The newDiffs that come after this spacer need to be moved down
			-- without overwriting the newDiffs that are already there
			local updatedNewDiffs = {}
			for lineNum, diffs in pairs(newDiffs) do
				if lineNum >= spacerLineNum then
					updatedNewDiffs[lineNum + 1] = diffs
				else
					updatedNewDiffs[lineNum] = diffs
				end
			end
			newDiffs = updatedNewDiffs
		end

		-- Update the maxLines after we may have inserted new lines
		maxLines = math.max(#richTextLinesOldString, #richTextLinesNewString)

		local removalScrollMarkers = {}
		local insertionScrollMarkers = {}
		for lineNum in oldDiffs do
			table.insert(
				removalScrollMarkers,
				e("Frame", {
					Size = UDim2.fromScale(0.5, 1 / maxLines),
					Position = UDim2.fromScale(0, (lineNum - 1) / maxLines),
					BorderSizePixel = 0,
					BackgroundColor3 = theme.Diff.Remove,
				})
			)
		end
		for lineNum in newDiffs do
			table.insert(
				insertionScrollMarkers,
				e("Frame", {
					Size = UDim2.fromScale(0.5, 1 / maxLines),
					Position = UDim2.fromScale(0.5, (lineNum - 1) / maxLines),
					BorderSizePixel = 0,
					BackgroundColor3 = theme.Diff.Add,
				})
			)
		end

		return e(BorderedContainer, {
			size = self.props.size,
			position = self.props.position,
			anchorPoint = self.props.anchorPoint,
			transparency = self.props.transparency,
		}, {
			Background = e("Frame", {
				Size = UDim2.new(1, 0, 1, 0),
				Position = UDim2.new(0, 0, 0, 0),
				BorderSizePixel = 0,
				BackgroundColor3 = self.scriptBackground,
				ZIndex = -10,
			}, {
				UICorner = e("UICorner", {
					CornerRadius = UDim.new(0, 5),
				}),
			}),
			Main = e("Frame", {
				Size = UDim2.new(1, -10, 1, -2),
				Position = UDim2.new(0, 2, 0, 2),
				BackgroundTransparency = 1,
				[Roact.Change.AbsoluteSize] = function(rbx)
					self.setWindowWidth(rbx.AbsoluteSize.X * 0.5 - 10)
				end,
			}, {
				Separator = e("Frame", {
					Size = UDim2.new(0, 2, 1, 0),
					Position = UDim2.new(0.5, 0, 0, 0),
					AnchorPoint = Vector2.new(0.5, 0),
					BorderSizePixel = 0,
					BackgroundColor3 = theme.BorderedContainer.BorderColor,
					BackgroundTransparency = 0.5,
				}),
				Old = e(VirtualScroller, {
					position = UDim2.new(0, 0, 0, 0),
					size = UDim2.new(0.5, -1, 1, 0),
					transparency = self.props.transparency,
					count = maxLines,
					updateEvent = self.updateEvent.Event,
					canvasWidth = canvasWidth,
					canvasPosition = self.canvasPosition,
					onCanvasPositionChanged = self.setCanvasPosition,
					render = function(i)
						if not richTextLinesOldString[i] then
							return e("ImageLabel", {
								Size = UDim2.fromScale(1, 1),
								Position = UDim2.fromScale(0, 0),
								BackgroundTransparency = 1,
								BorderSizePixel = 0,
								Image = Assets.Images.DiagonalLines,
								ImageTransparency = 0.7,
								ImageColor3 = theme.TextColor,
								ScaleType = Enum.ScaleType.Tile,
								TileSize = UDim2.new(0, 64, 4, 0),
							})
						end

						local lineDiffs = oldDiffs[i]
						local diffFrames = table.create(if lineDiffs then #lineDiffs else 0)

						if lineDiffs then
							local charWidth = math.round(theme.TextSize.Code * 0.5)
							for diffIdx, diff in lineDiffs do
								local start, stop = diff.start, diff.stop
								diffFrames[diffIdx] = e("Frame", {
									Size = UDim2.new(0, math.max(charWidth * (stop - start), charWidth * 0.4), 1, 0),
									Position = UDim2.fromOffset(charWidth * start, 0),
									BackgroundColor3 = theme.Diff.Remove,
									BackgroundTransparency = 0.75,
									BorderSizePixel = 0,
									ZIndex = -1,
								})
							end
						end

						return Roact.createFragment({
							CodeLabel = e("TextLabel", {
								Size = UDim2.fromScale(1, 1),
								Position = UDim2.fromScale(0, 0),
								Text = richTextLinesOldString[i],
								RichText = true,
								BackgroundColor3 = theme.Diff.Remove,
								BackgroundTransparency = if lineDiffs then 0.85 else 1,
								BorderSizePixel = 0,
								FontFace = theme.Font.Code,
								TextSize = theme.TextSize.Code,
								TextXAlignment = Enum.TextXAlignment.Left,
								TextYAlignment = Enum.TextYAlignment.Top,
								TextColor3 = Color3.fromRGB(255, 255, 255),
							}),
							DiffFrames = Roact.createFragment(diffFrames),
						})
					end,
					getHeightBinding = function()
						return self.lineHeight
					end,
				}),
				New = e(VirtualScroller, {
					position = UDim2.new(0.5, 1, 0, 0),
					size = UDim2.new(0.5, -1, 1, 0),
					transparency = self.props.transparency,
					count = maxLines,
					updateEvent = self.updateEvent.Event,
					canvasWidth = canvasWidth,
					canvasPosition = self.canvasPosition,
					onCanvasPositionChanged = self.setCanvasPosition,
					render = function(i)
						if not richTextLinesNewString[i] then
							return e("ImageLabel", {
								Size = UDim2.fromScale(1, 1),
								Position = UDim2.fromScale(0, 0),
								BackgroundTransparency = 1,
								BorderSizePixel = 0,
								Image = Assets.Images.DiagonalLines,
								ImageTransparency = 0.7,
								ImageColor3 = theme.TextColor,
								ScaleType = Enum.ScaleType.Tile,
								TileSize = UDim2.new(0, 64, 4, 0),
							})
						end

						local lineDiffs = newDiffs[i]
						local diffFrames = table.create(if lineDiffs then #lineDiffs else 0)

						if lineDiffs then
							local charWidth = math.round(theme.TextSize.Code * 0.5)
							for diffIdx, diff in lineDiffs do
								local start, stop = diff.start, diff.stop
								diffFrames[diffIdx] = e("Frame", {
									Size = UDim2.new(0, math.max(charWidth * (stop - start), charWidth * 0.4), 1, 0),
									Position = UDim2.fromOffset(charWidth * start, 0),
									BackgroundColor3 = theme.Diff.Add,
									BackgroundTransparency = 0.75,
									BorderSizePixel = 0,
									ZIndex = -1,
								})
							end
						end

						return Roact.createFragment({
							CodeLabel = e("TextLabel", {
								Size = UDim2.fromScale(1, 1),
								Position = UDim2.fromScale(0, 0),
								Text = richTextLinesNewString[i],
								RichText = true,
								BackgroundColor3 = theme.Diff.Add,
								BackgroundTransparency = if lineDiffs then 0.85 else 1,
								BorderSizePixel = 0,
								FontFace = theme.Font.Code,
								TextSize = theme.TextSize.Code,
								TextXAlignment = Enum.TextXAlignment.Left,
								TextYAlignment = Enum.TextYAlignment.Top,
								TextColor3 = Color3.fromRGB(255, 255, 255),
							}),
							DiffFrames = Roact.createFragment(diffFrames),
						})
					end,
					getHeightBinding = function()
						return self.lineHeight
					end,
				}),
			}),
			ScrollMarkers = e("Frame", {
				Size = self.windowWidth:map(function(windowWidth)
					return UDim2.new(0, 8, 1, -4 - (if canvasWidth > windowWidth then 10 else 0))
				end),
				Position = UDim2.new(1, -2, 0, 2),
				AnchorPoint = Vector2.new(1, 0),
				BackgroundTransparency = 1,
			}, {
				insertions = Roact.createFragment(insertionScrollMarkers),
				removals = Roact.createFragment(removalScrollMarkers),
			}),
		})
	end)
end

return StringDiffVisualizer
