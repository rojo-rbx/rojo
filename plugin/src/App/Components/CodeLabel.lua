local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)
local Highlighter = require(Packages.Highlighter)

local e = Roact.createElement

local CodeLabel = Roact.PureComponent:extend("CodeLabel")

function CodeLabel:init()
	self.labelRef = Roact.createRef()
	self.highlightsRef = Roact.createRef()

	self.themeConnection = nil
end

function CodeLabel:didMount()
    self:updateHighlighterTheme()
	self.themeConnection = settings():GetService("Studio").ThemeChanged:Connect(function()
		self:updateHighlighterTheme()
	end)

	Highlighter.highlight({
		textObject = self.labelRef:getValue(),
	})
	self:updateHighlights()
end

function CodeLabel:willUnmount()
	if self.themeConnection then
		self.themeConnection:Disconnect()
		self.themeConnection = nil
	end
end

function CodeLabel:didUpdate()
    self:updateHighlights()
end

function CodeLabel:updateHighlighterTheme()
	local studioTheme = settings():GetService("Studio").Theme
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

function CodeLabel:updateHighlights()
	local highlights = self.highlightsRef:getValue()
	if not highlights then
		return
	end

	for _, lineLabel in highlights:GetChildren() do
		local lineNum = tonumber(string.match(lineLabel.Name, "%d+") or "0")
		lineLabel.BackgroundColor3 = self.props.lineBackground
		lineLabel.BorderSizePixel = 0
		lineLabel.BackgroundTransparency = if self.props.markedLines[lineNum] then 0.25 else 1
	end
end

function CodeLabel:render()
	return e("TextLabel", {
		Size = self.props.size,
		Position = self.props.position,
		Text = self.props.text,
		BackgroundTransparency = 1,
		Font = Enum.Font.RobotoMono,
		TextSize = 16,
		TextXAlignment = Enum.TextXAlignment.Left,
		TextYAlignment = Enum.TextYAlignment.Top,
		TextColor3 = Color3.fromRGB(255, 255, 255),
		[Roact.Ref] = self.labelRef,
	}, {
		SyntaxHighlights = e("Folder", {
			[Roact.Ref] = self.highlightsRef,
		}),
	})
end

return CodeLabel
