local TextService = game:GetService("TextService")

local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)
local Log = require(Packages.Log)
local Highlighter = require(Packages.Highlighter)
local StringDiff = require(script:FindFirstChild("StringDiff"))

local Theme = require(Plugin.App.Theme)

local CodeLabel = require(Plugin.App.Components.CodeLabel)
local BorderedContainer = require(Plugin.App.Components.BorderedContainer)
local ScrollingFrame = require(Plugin.App.Components.ScrollingFrame)

local e = Roact.createElement

local StringDiffVisualizer = Roact.Component:extend("StringDiffVisualizer")

function StringDiffVisualizer:init()
	self.scriptBackground, self.setScriptBackground = Roact.createBinding(Color3.fromRGB(0, 0, 0))
	self.contentSize, self.setContentSize = Roact.createBinding(Vector2.new(0, 0))

	-- Ensure that the script background is up to date with the current theme
	self.themeChangedConnection = settings().Studio.ThemeChanged:Connect(function()
		task.defer(function()
			-- Defer to allow Highlighter to process the theme change first
			self:updateScriptBackground()
		end)
	end)

	self:calculateContentSize()
	self:updateScriptBackground()

	self:setState({
		add = {},
		remove = {},
	})
end

function StringDiffVisualizer:willUnmount()
	self.themeChangedConnection:Disconnect()
end

function StringDiffVisualizer:updateScriptBackground()
	local backgroundColor = Highlighter.getTokenColor("background")
	if backgroundColor ~= self.scriptBackground:getValue() then
		self.setScriptBackground(backgroundColor)
	end
end

function StringDiffVisualizer:didUpdate(previousProps)
	if previousProps.oldText ~= self.props.oldText or previousProps.newText ~= self.props.newText then
		self:calculateContentSize()
		local add, remove = self:calculateDiffLines()
		self:setState({
			add = add,
			remove = remove,
		})
	end
end

function StringDiffVisualizer:calculateContentSize()
	local oldText, newText = self.props.oldText, self.props.newText

	local oldTextBounds = TextService:GetTextSize(oldText, 16, Enum.Font.RobotoMono, Vector2.new(99999, 99999))
	local newTextBounds = TextService:GetTextSize(newText, 16, Enum.Font.RobotoMono, Vector2.new(99999, 99999))

	self.setContentSize(
		Vector2.new(math.max(oldTextBounds.X, newTextBounds.X), math.max(oldTextBounds.Y, newTextBounds.Y))
	)
end

function StringDiffVisualizer:calculateDiffLines()
	local oldText, newText = self.props.oldText, self.props.newText

	-- Diff the two texts
	local startClock = os.clock()
	local diffs = StringDiff.findDiffs(oldText, newText)
	local stopClock = os.clock()

	Log.trace(
		"Diffing {} byte and {} byte strings took {} microseconds and found {} diff sections",
		#oldText,
		#newText,
		math.round((stopClock - startClock) * 1000 * 1000),
		#diffs
	)

	-- Determine which lines to highlight
	local add, remove = {}, {}

	local oldLineNum, newLineNum = 1, 1
	for _, diff in diffs do
		local actionType, text = diff.actionType, diff.value
		local lines = select(2, string.gsub(text, "\n", "\n"))

		if actionType == StringDiff.ActionTypes.Equal then
			oldLineNum += lines
			newLineNum += lines
		elseif actionType == StringDiff.ActionTypes.Insert then
			if lines > 0 then
				local textLines = string.split(text, "\n")
				for i, textLine in textLines do
					if string.match(textLine, "%S") then
						add[newLineNum + i - 1] = true
					end
				end
			else
				if string.match(text, "%S") then
					add[newLineNum] = true
				end
			end
			newLineNum += lines
		elseif actionType == StringDiff.ActionTypes.Delete then
			if lines > 0 then
				local textLines = string.split(text, "\n")
				for i, textLine in textLines do
					if string.match(textLine, "%S") then
						remove[oldLineNum + i - 1] = true
					end
				end
			else
				if string.match(text, "%S") then
					remove[oldLineNum] = true
				end
			end
			oldLineNum += lines
		else
			Log.warn("Unknown diff action: {} {}", actionType, text)
		end
	end

	return add, remove
end

function StringDiffVisualizer:render()
	local oldText, newText = self.props.oldText, self.props.newText

	return Theme.with(function(theme)
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
			Old = e(ScrollingFrame, {
				position = UDim2.new(0, 2, 0, 2),
				size = UDim2.new(0.5, -7, 1, -4),
				scrollingDirection = Enum.ScrollingDirection.XY,
				transparency = self.props.transparency,
				contentSize = self.contentSize,
			}, {
				Source = e(CodeLabel, {
					size = UDim2.new(1, 0, 1, 0),
					position = UDim2.new(0, 0, 0, 0),
					text = oldText,
					lineBackground = theme.Diff.Remove,
					markedLines = self.state.remove,
				}),
			}),
			New = e(ScrollingFrame, {
				position = UDim2.new(0.5, 5, 0, 2),
				size = UDim2.new(0.5, -7, 1, -4),
				scrollingDirection = Enum.ScrollingDirection.XY,
				transparency = self.props.transparency,
				contentSize = self.contentSize,
			}, {
				Source = e(CodeLabel, {
					size = UDim2.new(1, 0, 1, 0),
					position = UDim2.new(0, 0, 0, 0),
					text = newText,
					lineBackground = theme.Diff.Add,
					markedLines = self.state.add,
				}),
			}),
		})
	end)
end

return StringDiffVisualizer
