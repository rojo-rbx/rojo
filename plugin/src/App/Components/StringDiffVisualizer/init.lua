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
		currentDiffs = {},
		incomingDiffs = {},
		currentSpacers = {},
		incomingSpacers = {},
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
	if
		previousProps.currentString ~= self.props.currentString
		or previousProps.incomingString ~= self.props.incomingString
	then
		self:updateDiffs()
	end
end

function StringDiffVisualizer:calculateContentSize(theme)
	local currentString, incomingString = self.props.currentString, self.props.incomingString

	local currentStringBounds = getTextBoundsAsync(currentString, theme.Font.Code, theme.TextSize.Code, math.huge)
	local incomingStringBounds = getTextBoundsAsync(incomingString, theme.Font.Code, theme.TextSize.Code, math.huge)

	return Vector2.new(
		math.max(currentStringBounds.X, incomingStringBounds.X),
		math.max(currentStringBounds.Y, incomingStringBounds.Y)
	)
end

function StringDiffVisualizer:updateDiffs()
	Timer.start("StringDiffVisualizer:updateDiffs")
	local currentString, incomingString = self.props.currentString, self.props.incomingString

	-- Diff the two texts
	local startClock = os.clock()
	local diffs =
		StringDiff.findDiffs((string.gsub(currentString, "\t", "    ")), (string.gsub(incomingString, "\t", "    ")))
	local stopClock = os.clock()

	Log.trace(
		"Diffing {} byte and {} byte strings took {} microseconds and found {} diff sections",
		#currentString,
		#incomingString,
		math.round((stopClock - startClock) * 1000 * 1000),
		#diffs
	)

	-- Find the diff locations
	local currentDiffs, incomingDiffs = {}, {}
	local currentSpacers, incomingSpacers = {}, {}

	local firstDiffLineNum = 0

	local currentLineNum, currentIdx, incomingLineNum, incomingIdx = 1, 0, 1, 0
	for _, diff in diffs do
		local actionType, text = diff.actionType, diff.value
		local lines = string.split(text, "\n")

		if actionType == StringDiff.ActionTypes.Equal then
			for i, line in lines do
				if i > 1 then
					currentLineNum += 1
					currentIdx = 0
					incomingLineNum += 1
					incomingIdx = 0
				end
				currentIdx += #line
				incomingIdx += #line
			end
		elseif actionType == StringDiff.ActionTypes.Insert then
			if firstDiffLineNum == 0 then
				firstDiffLineNum = incomingLineNum
			end

			for i, line in lines do
				if i > 1 then
					incomingLineNum += 1
					incomingIdx = 0

					table.insert(currentSpacers, { currentLineNum = currentLineNum, incomingLineNum = incomingLineNum })
				end
				if not incomingDiffs[incomingLineNum] then
					incomingDiffs[incomingLineNum] = {
						{ start = incomingIdx, stop = incomingIdx + #line },
					}
				else
					table.insert(incomingDiffs[incomingLineNum], {
						start = incomingIdx,
						stop = incomingIdx + #line,
					})
				end
				incomingIdx += #line
			end
		elseif actionType == StringDiff.ActionTypes.Delete then
			if firstDiffLineNum == 0 then
				firstDiffLineNum = currentLineNum
			end

			for i, line in lines do
				if i > 1 then
					currentLineNum += 1
					currentIdx = 0

					table.insert(
						incomingSpacers,
						{ currentLineNum = currentLineNum, incomingLineNum = incomingLineNum }
					)
				end
				if not currentDiffs[currentLineNum] then
					currentDiffs[currentLineNum] = {
						{ start = currentIdx, stop = currentIdx + #line },
					}
				else
					table.insert(currentDiffs[currentLineNum], {
						start = currentIdx,
						stop = currentIdx + #line,
					})
				end
				currentIdx += #line
			end
		else
			Log.warn("Unknown diff action: {} {}", actionType, text)
		end
	end

	-- Filter out diffs that are just incominglines being added/removed from existing non-empty lines.
	-- This is done to make the diff visualization less noisy.

	local currentStringLines = string.split(currentString, "\n")
	local incomingStringLines = string.split(incomingString, "\n")

	for lineNum, lineDiffs in currentDiffs do
		if
			(#lineDiffs > 1) -- Not just incomingline
			or (lineDiffs[1].start ~= lineDiffs[1].stop) -- Not a incomingline at all
			or (currentStringLines[lineNum] == "") -- Empty line, so the incomingline change is significant
		then
			continue
		end
		-- Just a noisy incomingline diff, clear it
		currentDiffs[lineNum] = nil
	end

	for lineNum, lineDiffs in incomingDiffs do
		if
			(#lineDiffs > 1) -- Not just incomingline
			or (lineDiffs[1].start ~= lineDiffs[1].stop) -- Not a incomingline at all
			or (incomingStringLines[lineNum] == "") -- Empty line, so the incomingline change is significant
		then
			continue
		end
		-- Just a noisy incomingline diff, clear it
		incomingDiffs[lineNum] = nil
	end

	Timer.stop()

	self:setState({
		currentDiffs = currentDiffs,
		incomingDiffs = incomingDiffs,
		currentSpacers = currentSpacers,
		incomingSpacers = incomingSpacers,
	})
	-- Scroll to the first diff line
	self.setCanvasPosition(Vector2.new(0, math.max(0, (firstDiffLineNum - 4) * 16)))
end

function StringDiffVisualizer:render()
	local currentString, incomingString = self.props.currentString, self.props.incomingString
	local currentDiffs, incomingDiffs = self.state.currentDiffs, self.state.incomingDiffs
	local currentSpacers, incomingSpacers = self.state.currentSpacers, self.state.incomingSpacers

	return Theme.with(function(theme)
		self.setLineHeight(theme.TextSize.Code)

		local richTextLinesCurrentString = Highlighter.buildRichTextLines({
			src = currentString,
		})
		local richTextLinesIncomingString = Highlighter.buildRichTextLines({
			src = incomingString,
		})

		local maxLines = math.max(#richTextLinesCurrentString, #richTextLinesIncomingString)

		-- Calculate the width of the canvas
		-- (One line at a time to avoid the 200k char limit of getTextBoundsAsync)
		local canvasWidth = 0
		for i = 1, maxLines do
			local currentLine = richTextLinesCurrentString[i]
			if currentLine and currentLine ~= "" then
				local bounds = getTextBoundsAsync(currentLine, theme.Font.Code, theme.TextSize.Code, math.huge, true)
				if bounds.X > canvasWidth then
					canvasWidth = bounds.X
				end
			end
			local incomingLine = richTextLinesIncomingString[i]
			if incomingLine and currentLine ~= "" then
				local bounds = getTextBoundsAsync(incomingLine, theme.Font.Code, theme.TextSize.Code, math.huge, true)
				if bounds.X > canvasWidth then
					canvasWidth = bounds.X
				end
			end
		end

		-- Adjust the rich text lines and their diffs to include spacers (aka nil lines)
		for spacerIdx, spacer in currentSpacers do
			local spacerLineNum = spacer.currentLineNum + (spacerIdx - 1)
			table.insert(richTextLinesCurrentString, spacerLineNum, nil)
			-- The currentDiffs that come after this spacer need to be moved down
			-- without overwriting the currentDiffs that are already there
			local updatedCurrentDiffs = {}
			for lineNum, diffs in pairs(currentDiffs) do
				if lineNum >= spacerLineNum then
					updatedCurrentDiffs[lineNum + 1] = diffs
				else
					updatedCurrentDiffs[lineNum] = diffs
				end
			end
			currentDiffs = updatedCurrentDiffs
		end
		for spacerIdx, spacer in incomingSpacers do
			local spacerLineNum = spacer.incomingLineNum + (spacerIdx - 1)
			table.insert(richTextLinesIncomingString, spacerLineNum, nil)
			-- The incomingDiffs that come after this spacer need to be moved down
			-- without overwriting the incomingDiffs that are already there
			local updatedIncomingDiffs = {}
			for lineNum, diffs in pairs(incomingDiffs) do
				if lineNum >= spacerLineNum then
					updatedIncomingDiffs[lineNum + 1] = diffs
				else
					updatedIncomingDiffs[lineNum] = diffs
				end
			end
			incomingDiffs = updatedIncomingDiffs
		end

		-- Update the maxLines after we may have inserted additional lines
		maxLines = math.max(#richTextLinesCurrentString, #richTextLinesIncomingString)

		local removalScrollMarkers = {}
		local insertionScrollMarkers = {}
		for lineNum in currentDiffs do
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
		for lineNum in incomingDiffs do
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
				Current = e(VirtualScroller, {
					position = UDim2.new(0, 0, 0, 0),
					size = UDim2.new(0.5, -1, 1, 0),
					transparency = self.props.transparency,
					count = maxLines,
					updateEvent = self.updateEvent.Event,
					canvasWidth = canvasWidth,
					canvasPosition = self.canvasPosition,
					onCanvasPositionChanged = self.setCanvasPosition,
					render = function(i)
						if not richTextLinesCurrentString[i] then
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

						local lineDiffs = currentDiffs[i]
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
								Text = richTextLinesCurrentString[i],
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
				Incoming = e(VirtualScroller, {
					position = UDim2.new(0.5, 1, 0, 0),
					size = UDim2.new(0.5, -1, 1, 0),
					transparency = self.props.transparency,
					count = maxLines,
					updateEvent = self.updateEvent.Event,
					canvasWidth = canvasWidth,
					canvasPosition = self.canvasPosition,
					onCanvasPositionChanged = self.setCanvasPosition,
					render = function(i)
						if not richTextLinesIncomingString[i] then
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

						local lineDiffs = incomingDiffs[i]
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
								Text = richTextLinesIncomingString[i],
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
