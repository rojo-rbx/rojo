local Rojo = script:FindFirstAncestor("Rojo")
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)
local Highlighter = require(Packages.Highlighter)
Highlighter.matchStudioSettings()

local e = Roact.createElement

local CodeLabel = Roact.PureComponent:extend("CodeLabel")

function CodeLabel:init()
	self.labelRef = Roact.createRef()
	self.highlightsRef = Roact.createRef()
end

function CodeLabel:didMount()
	Highlighter.highlight({
		textObject = self.labelRef:getValue(),
	})
	self:updateHighlights()
end

function CodeLabel:didUpdate()
	self:updateHighlights()
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
