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

	-- Ensure that the script background is up to date with the current theme
	self.themeChangedConnection = settings().Studio.ThemeChanged:Connect(function()
		task.defer(function()
			-- Defer to allow Highlighter to process the theme change first
			self:updateScriptBackground()
		end)
	end)

	self:updateScriptBackground()

	self:setState({
		oldDiffs = {},
		newDiffs = {},
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
		local oldDiffs, newDiffs = self:calculateDiffs()
		self:setState({
			oldDiffs = oldDiffs,
			newDiffs = newDiffs,
		})
	end
end

function StringDiffVisualizer:calculateContentSize(theme)
	local oldString, newString = self.props.oldString, self.props.newString

	local oldStringBounds = getTextBoundsAsync(oldString, theme.Font.Code, theme.TextSize.Code, math.huge)
	local newStringBounds = getTextBoundsAsync(newString, theme.Font.Code, theme.TextSize.Code, math.huge)

	return Vector2.new(math.max(oldStringBounds.X, newStringBounds.X), math.max(oldStringBounds.Y, newStringBounds.Y))
end

function StringDiffVisualizer:calculateDiffs()
	Timer.start("StringDiffVisualizer:calculateDiffs")
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
			for i, line in lines do
				if i > 1 then
					newLineNum += 1
					newIdx = 0
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
			for i, line in lines do
				if i > 1 then
					oldLineNum += 1
					oldIdx = 0
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

	Timer.stop()
	return oldDiffs, newDiffs
end

function StringDiffVisualizer:render()
	local oldString, newString = self.props.oldString, self.props.newString
	local oldDiffs, newDiffs = self.state.oldDiffs, self.state.newDiffs

	return Theme.with(function(theme)
		self.setLineHeight(theme.TextSize.Code)

		local contentSize = self:calculateContentSize(theme)

		local richTextLinesOldString = Highlighter.buildRichTextLines({
			src = oldString,
		})
		local richTextLinesNewString = Highlighter.buildRichTextLines({
			src = newString,
		})

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
			Separator = e("Frame", {
				Size = UDim2.new(0, 2, 1, 0),
				Position = UDim2.new(0.5, 0, 0, 0),
				AnchorPoint = Vector2.new(0.5, 0),
				BorderSizePixel = 0,
				BackgroundColor3 = theme.BorderedContainer.BorderColor,
				BackgroundTransparency = 0.5,
			}),
			Old = e(VirtualScroller, {
				position = UDim2.new(0, 2, 0, 2),
				size = UDim2.new(0.5, -7, 1, -4),
				transparency = self.props.transparency,
				count = #richTextLinesOldString,
				updateEvent = self.updateEvent.Event,
				canvasWidth = contentSize.X,
				render = function(i)
					local lineDiffs = oldDiffs[i]
					local diffFrames = table.create(if lineDiffs then #lineDiffs else 0)

					if lineDiffs then
						local charWidth = math.round(theme.TextSize.Code * 0.5)
						for diffIdx, diff in lineDiffs do
							local start, stop = diff.start, diff.stop
							diffFrames[diffIdx] = e("Frame", {
								Size = UDim2.new(0, math.max(charWidth * (stop - start), charWidth / 2), 1, 0),
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
				position = UDim2.new(0.5, 5, 0, 2),
				size = UDim2.new(0.5, -7, 1, -4),
				transparency = self.props.transparency,
				count = #richTextLinesNewString,
				updateEvent = self.updateEvent.Event,
				canvasWidth = contentSize.X,
				render = function(i)
					local lineDiffs = newDiffs[i]
					local diffFrames = table.create(if lineDiffs then #lineDiffs else 0)

					if lineDiffs then
						local charWidth = math.round(theme.TextSize.Code * 0.5)
						for diffIdx, diff in lineDiffs do
							local start, stop = diff.start, diff.stop
							diffFrames[diffIdx] = e("Frame", {
								Size = UDim2.new(0, math.max(charWidth * (stop - start), charWidth / 2), 1, 0),
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
		})
	end)
end

return StringDiffVisualizer
