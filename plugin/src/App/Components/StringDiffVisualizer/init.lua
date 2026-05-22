local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)
local Log = require(Packages.Log)
local Highlighter = require(Packages.Highlighter)
Highlighter.matchStudioSettings()
local StringDiff = require(script:FindFirstChild("StringDiff"))

local Timer = require(Plugin.Timer)
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
		-- Delay to allow Highlighter to process the theme change first
		task.delay(1 / 20, function()
			self:updateScriptBackground()
			self:updateDiffs()
			-- Rerender the virtual list elements
			self.updateEvent:Fire()
		end)
	end)

	self:updateScriptBackground()
	self:updateDiffs()
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

	-- Build the rich text lines
	local currentRichTextLines = Highlighter.buildRichTextLines({
		src = currentString,
	})
	local incomingRichTextLines = Highlighter.buildRichTextLines({
		src = incomingString,
	})

	local maxLines = math.max(#currentRichTextLines, #incomingRichTextLines)

	-- Find the diff locations
	local currentDiffs, incomingDiffs = {}, {}
	local firstDiffLineNum = 0

	local currentLineNum, incomingLineNum = 1, 1
	local currentIdx, incomingIdx = 1, 1
	for _, diff in diffs do
		local actionType, text = diff.actionType, diff.value
		local lineCount = select(2, string.gsub(text, "\n", "\n"))
		local lines = string.split(text, "\n")

		if actionType == StringDiff.ActionTypes.Equal then
			if lineCount > 0 then
				-- Jump cursor ahead to last line
				currentLineNum += lineCount
				incomingLineNum += lineCount
				currentIdx = #lines[#lines]
				incomingIdx = #lines[#lines]
			else
				-- Move along this line
				currentIdx += #text
				incomingIdx += #text
			end

			continue
		end

		if actionType == StringDiff.ActionTypes.Insert then
			if firstDiffLineNum == 0 then
				firstDiffLineNum = incomingLineNum
			end

			for i, lineText in lines do
				if i > 1 then
					-- Move to next line
					incomingLineNum += 1
					incomingIdx = 0
				end
				if not incomingDiffs[incomingLineNum] then
					incomingDiffs[incomingLineNum] = {}
				end
				-- Mark these characters on this line
				table.insert(incomingDiffs[incomingLineNum], {
					start = incomingIdx,
					stop = incomingIdx + #lineText,
				})
				incomingIdx += #lineText
			end
		elseif actionType == StringDiff.ActionTypes.Delete then
			if firstDiffLineNum == 0 then
				firstDiffLineNum = currentLineNum
			end

			for i, lineText in lines do
				if i > 1 then
					-- Move to next line
					currentLineNum += 1
					currentIdx = 0
				end
				if not currentDiffs[currentLineNum] then
					currentDiffs[currentLineNum] = {}
				end
				-- Mark these characters on this line
				table.insert(currentDiffs[currentLineNum], {
					start = currentIdx,
					stop = currentIdx + #lineText,
				})
				currentIdx += #lineText
			end
		else
			Log.warn("Unknown diff action: {} {}", actionType, text)
		end
	end

	Timer.stop()

	self:setState({
		maxLines = maxLines,
		currentRichTextLines = currentRichTextLines,
		incomingRichTextLines = incomingRichTextLines,
		currentDiffs = currentDiffs,
		incomingDiffs = incomingDiffs,
	})

	-- Scroll to the first diff line
	task.defer(self.setCanvasPosition, Vector2.new(0, math.max(0, (firstDiffLineNum - 4) * 16)))
end

function StringDiffVisualizer:render()
	local currentDiffs, incomingDiffs = self.state.currentDiffs, self.state.incomingDiffs
	local currentRichTextLines, incomingRichTextLines =
		self.state.currentRichTextLines, self.state.incomingRichTextLines
	local maxLines = self.state.maxLines

	return Theme.with(function(theme)
		self.setLineHeight(theme.TextSize.Code)

		-- Calculate the width of the canvas
		-- (One line at a time to avoid the char limit of getTextBoundsAsync)
		local canvasWidth = 0
		for i = 1, maxLines do
			local currentLine = currentRichTextLines[i]
			if currentLine and string.find(currentLine, "%S") then
				local bounds = getTextBoundsAsync(currentLine, theme.Font.Code, theme.TextSize.Code, math.huge, true)
				if bounds.X > canvasWidth then
					canvasWidth = bounds.X
				end
			end
			local incomingLine = incomingRichTextLines[i]
			if incomingLine and string.find(incomingLine, "%S") then
				local bounds = getTextBoundsAsync(incomingLine, theme.Font.Code, theme.TextSize.Code, math.huge, true)
				if bounds.X > canvasWidth then
					canvasWidth = bounds.X
				end
			end
		end

		local lineNumberWidth =
			getTextBoundsAsync(tostring(maxLines), theme.Font.Code, theme.TextSize.Body, math.huge, true).X

		canvasWidth += lineNumberWidth + 12

		local removalScrollMarkers = {}
		local insertionScrollMarkers = {}
		for lineNum in currentDiffs do
			table.insert(
				removalScrollMarkers,
				e("Frame", {
					Size = UDim2.fromScale(0.5, 1 / maxLines),
					Position = UDim2.fromScale(0, (lineNum - 1) / maxLines),
					BorderSizePixel = 0,
					BackgroundColor3 = theme.Diff.Background.Remove,
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
					BackgroundColor3 = theme.Diff.Background.Add,
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
						local lineDiffs = currentDiffs[i]
						local diffFrames = table.create(if lineDiffs then #lineDiffs else 0)

						-- Show diff markers over the specific changed characters
						if lineDiffs then
							local charWidth = math.round(theme.TextSize.Code * 0.5)
							for diffIdx, diff in lineDiffs do
								local start, stop = diff.start, diff.stop
								diffFrames[diffIdx] = e("Frame", {
									Size = if #lineDiffs == 1
											and start == 0
											and stop == 0
										then UDim2.fromScale(1, 1)
										else UDim2.new(
											0,
											math.max(charWidth * (stop - start), charWidth * 0.4),
											1,
											0
										),
									Position = UDim2.fromOffset(charWidth * start, 0),
									BackgroundColor3 = theme.Diff.Background.Remove,
									BackgroundTransparency = 0.85,
									BorderSizePixel = 0,
									ZIndex = -1,
								})
							end
						end

						return Roact.createFragment({
							LineNumber = e("TextLabel", {
								Size = UDim2.new(0, lineNumberWidth + 8, 1, 0),
								Text = i,
								BackgroundColor3 = Color3.new(0, 0, 0),
								BackgroundTransparency = 0.9,
								BorderSizePixel = 0,
								FontFace = theme.Font.Code,
								TextSize = theme.TextSize.Body,
								TextColor3 = if lineDiffs then theme.Diff.Background.Remove else theme.SubTextColor,
								TextXAlignment = Enum.TextXAlignment.Right,
							}, {
								Padding = e("UIPadding", { PaddingRight = UDim.new(0, 6) }),
							}),
							Content = e("Frame", {
								Size = UDim2.new(1, -(lineNumberWidth + 10), 1, 0),
								Position = UDim2.fromScale(1, 0),
								AnchorPoint = Vector2.new(1, 0),
								BackgroundColor3 = theme.Diff.Background.Remove,
								BackgroundTransparency = if lineDiffs then 0.95 else 1,
								BorderSizePixel = 0,
							}, {
								CodeLabel = e("TextLabel", {
									Size = UDim2.fromScale(1, 1),
									Position = UDim2.fromScale(0, 0),
									Text = currentRichTextLines[i] or "",
									RichText = true,
									BackgroundTransparency = 1,
									BorderSizePixel = 0,
									FontFace = theme.Font.Code,
									TextSize = theme.TextSize.Code,
									TextXAlignment = Enum.TextXAlignment.Left,
									TextYAlignment = Enum.TextYAlignment.Top,
									TextColor3 = Color3.fromRGB(255, 255, 255),
								}),
								DiffFrames = Roact.createFragment(diffFrames),
							}),
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
						local lineDiffs = incomingDiffs[i]
						local diffFrames = table.create(if lineDiffs then #lineDiffs else 0)

						-- Show diff markers over the specific changed characters
						if lineDiffs then
							local charWidth = math.round(theme.TextSize.Code * 0.5)
							for diffIdx, diff in lineDiffs do
								local start, stop = diff.start, diff.stop
								diffFrames[diffIdx] = e("Frame", {
									Size = if #lineDiffs == 1
											and start == 0
											and stop == 0
										then UDim2.fromScale(1, 1)
										else UDim2.new(
											0,
											math.max(charWidth * (stop - start), charWidth * 0.4),
											1,
											0
										),
									Position = UDim2.fromOffset(charWidth * start, 0),
									BackgroundColor3 = theme.Diff.Background.Add,
									BackgroundTransparency = 0.85,
									BorderSizePixel = 0,
									ZIndex = -1,
								})
							end
						end

						return Roact.createFragment({
							LineNumber = e("TextLabel", {
								Size = UDim2.new(0, lineNumberWidth + 8, 1, 0),
								Text = i,
								BackgroundColor3 = Color3.new(0, 0, 0),
								BackgroundTransparency = 0.9,
								BorderSizePixel = 0,
								FontFace = theme.Font.Code,
								TextSize = theme.TextSize.Body,
								TextColor3 = if lineDiffs then theme.Diff.Background.Add else theme.SubTextColor,
								TextXAlignment = Enum.TextXAlignment.Right,
							}, {
								Padding = e("UIPadding", { PaddingRight = UDim.new(0, 6) }),
							}),
							Content = e("Frame", {
								Size = UDim2.new(1, -(lineNumberWidth + 10), 1, 0),
								Position = UDim2.fromScale(1, 0),
								AnchorPoint = Vector2.new(1, 0),
								BackgroundColor3 = theme.Diff.Background.Add,
								BackgroundTransparency = if lineDiffs then 0.95 else 1,
								BorderSizePixel = 0,
							}, {
								CodeLabel = e("TextLabel", {
									Size = UDim2.fromScale(1, 1),
									Position = UDim2.fromScale(0, 0),
									Text = incomingRichTextLines[i] or "",
									RichText = true,
									BackgroundColor3 = theme.Diff.Background.Add,
									BackgroundTransparency = 1,
									BorderSizePixel = 0,
									FontFace = theme.Font.Code,
									TextSize = theme.TextSize.Code,
									TextXAlignment = Enum.TextXAlignment.Left,
									TextYAlignment = Enum.TextYAlignment.Top,
									TextColor3 = Color3.fromRGB(255, 255, 255),
								}),
								DiffFrames = Roact.createFragment(diffFrames),
							}),
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
