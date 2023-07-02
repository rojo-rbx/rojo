local TextService = game:GetService("TextService")

local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)
local Log = require(Packages.Log)
local Highlighter = require(Packages.Highlighter)
local DMP = require(script:FindFirstChild("DiffMatchPatch"))

local Theme = require(Plugin.App.Theme)

local BorderedContainer = require(Plugin.App.Components.BorderedContainer)
local ScrollingFrame = require(Plugin.App.Components.ScrollingFrame)

local e = Roact.createElement

local StringDiffVisualizer = Roact.Component:extend("StringDiffVisualizer")

settings().Studio.Theme:GetColor(Enum.StudioStyleGuideColor.MainButton, Enum.StudioStyleGuideModifier.Disabled)

function StringDiffVisualizer:init()
	self.scriptBackground, self.setScriptBackground = Roact.createBinding(Color3.fromRGB(0, 0, 0))
	self.contentSize, self.setContentSize = Roact.createBinding(Vector2.new(0, 0))

	self.oldHighlights = Roact.createRef()
	self.newHighlights = Roact.createRef()

	self.themeConnection = nil
	self:updateHighlighterTheme()

	self:calculateContentSize()
end

function StringDiffVisualizer:didMount()
	self.themeConnection = settings():GetService("Studio").ThemeChanged:Connect(function()
		self:updateHighlighterTheme()
	end)
end

function StringDiffVisualizer:willUnmount()
	self.themeConnection:Disconnect()
end

function StringDiffVisualizer:updateHighlighterTheme()
	local studioTheme = settings():GetService("Studio").Theme
	self.setScriptBackground(studioTheme:GetColor(Enum.StudioStyleGuideColor.ScriptBackground))
	Highlighter.setTokenColors({
		["iden"] = studioTheme:GetColor(Enum.StudioStyleGuideColor.ScriptText),
		["keyword"] = studioTheme:GetColor(Enum.StudioStyleGuideColor.ScriptKeyword),
		["builtin"] = studioTheme:GetColor(Enum.StudioStyleGuideColor.ScriptBuiltInFunction),
		["string"] = studioTheme:GetColor(Enum.StudioStyleGuideColor.ScriptString),
		["number"] = studioTheme:GetColor(Enum.StudioStyleGuideColor.ScriptNumber),
		["comment"] = studioTheme:GetColor(Enum.StudioStyleGuideColor.ScriptComment),
		["operator"] = studioTheme:GetColor(Enum.StudioStyleGuideColor.ScriptOperator),
	})
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
	local diffs = DMP.diff_main(oldText, newText)
	DMP.diff_cleanupSemantic(diffs)
	local stopClock = os.clock()

	Log.trace("Diffing {} byte and {} byte strings took {} microseconds and found {} diffs", #oldText, #newText, math.round((stopClock - startClock) * 1000 * 1000), #diffs)

	-- Determine which lines to highlight
	local add, remove = {}, {}

	local oldLineNum, newLineNum = 1, 1
	for _, diff in diffs do
		local action, text = diff[1], diff[2]
		local lines = select(2, string.gsub(text, "\n", "\n"))

		if action == DMP.DIFF_EQUAL then
			oldLineNum += lines
			newLineNum += lines
		elseif action == DMP.DIFF_INSERT then
			for i = newLineNum, newLineNum + lines do
				add[i] = true
			end
			newLineNum += lines
		elseif action == DMP.DIFF_DELETE then
			for i = oldLineNum, oldLineNum + lines do
				remove[i] = true
			end
			oldLineNum += lines
		else
			Log.warn("Unknown diff action: {} {}", action, text)
		end
	end

	if self.oldHighlights.current then
		for _, lineLabel in self.oldHighlights.current:GetChildren() do
			lineLabel.BackgroundTransparency =
				if remove[tonumber(string.match(lineLabel.Name, "%d+"))] then 0.3 else 1
		end
	end

	if self.newHighlights.current then
		for _, lineLabel in self.newHighlights.current:GetChildren() do
			lineLabel.BackgroundTransparency =
				if add[tonumber(string.match(lineLabel.Name, "%d+"))] then 0.3 else 1
		end
	end

	return add, remove
end

function StringDiffVisualizer:willUpdate()
	self:calculateContentSize()
end

function StringDiffVisualizer:render()
	local oldText, newText = self.props.oldText, self.props.newText

	return Theme.with(function(theme)
		if self.newHighlights.current then
			for _, lineLabel in self.newHighlights.current:GetChildren() do
				lineLabel.BackgroundColor3 = theme.Diff.Add
				lineLabel.BorderSizePixel = 0
			end
		end
		if self.oldHighlights.current then
			for _, lineLabel in self.oldHighlights.current:GetChildren() do
				lineLabel.BackgroundColor3 = theme.Diff.Remove
				lineLabel.BorderSizePixel = 0
			end
		end

		self:calculateDiffLines()

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
				})
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
				TextLabel = e("TextLabel", {
					Size = UDim2.new(1, 0, 1, 0),
					BackgroundTransparency = 1,
					Text = oldText,
					Font = Enum.Font.RobotoMono,
					TextSize = 16,
					TextXAlignment = Enum.TextXAlignment.Left,
					TextYAlignment = Enum.TextYAlignment.Top,
					TextColor3 = Color3.fromRGB(255, 255, 255),
					[Roact.Event.AncestryChanged] = function(rbx)
						if rbx:IsDescendantOf(game) then
							Highlighter.highlight({
								textObject = rbx,
							})
						end
					end,
				}, {
					SyntaxHighlights = e("Folder", {
						[Roact.Ref] = self.oldHighlights,
					}),
				}),
			}),
			New = e(ScrollingFrame, {
				position = UDim2.new(0.5, 5, 0, 2),
				size = UDim2.new(0.5, -7, 1, -4),
				scrollingDirection = Enum.ScrollingDirection.XY,
				transparency = self.props.transparency,
				contentSize = self.contentSize,
			}, {
				TextLabel = e("TextLabel", {
					Size = UDim2.new(1, 0, 1, 0),
					BackgroundTransparency = 1,
					Text = newText,
					Font = Enum.Font.RobotoMono,
					TextSize = 16,
					TextXAlignment = Enum.TextXAlignment.Left,
					TextYAlignment = Enum.TextYAlignment.Top,
					TextColor3 = Color3.fromRGB(255, 255, 255),
					[Roact.Event.AncestryChanged] = function(rbx)
						if rbx:IsDescendantOf(game) then
							Highlighter.highlight({
								textObject = rbx,
							})
						end
					end,
				}, {
					SyntaxHighlights = e("Folder", {
						[Roact.Ref] = self.newHighlights,
					}),
				}),
			}),
		})
	end)
end

return StringDiffVisualizer
