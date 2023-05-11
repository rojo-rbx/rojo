local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local Assets = require(Plugin.Assets)
local Theme = require(Plugin.App.Theme)
local bindingUtil = require(Plugin.App.bindingUtil)

local e = Roact.createElement

local VirtualScroller = Roact.Component:extend("VirtualScroller")

function VirtualScroller:init()
	self.scrollFrameRef = Roact.createRef()
	self:setState({
		WindowSize = Vector2.new(),
		CanvasPosition = Vector2.new(),
	})

	self.totalCanvas, self.setTotalCanvas = Roact.createBinding(0)
	self.padding, self.setPadding = Roact.createBinding(0)

	self:refresh()
	if self.props.updateEvent then
		self.connection = self.props.updateEvent:Connect(function()
			self:refresh()
		end)
	end
end

function VirtualScroller:didMount()
	local rbx = self.scrollFrameRef:getValue()

	local windowSizeSignal = rbx:GetPropertyChangedSignal("AbsoluteWindowSize")
	self.windowSizeChanged = windowSizeSignal:Connect(function()
		self:setState({ WindowSize = rbx.AbsoluteWindowSize })
		self:refresh()
	end)

	local canvasPositionSignal = rbx:GetPropertyChangedSignal("CanvasPosition")
	self.canvasPositionChanged = canvasPositionSignal:Connect(function()
		if math.abs(rbx.CanvasPosition.Y - self.state.CanvasPosition.Y) > 5 then
			self:setState({ CanvasPosition = rbx.CanvasPosition })
			self:refresh()
		end
	end)

	self:refresh()
end

function VirtualScroller:willUnmount()
	self.windowSizeChanged:Disconnect()
	self.canvasPositionChanged:Disconnect()
	if self.connection then
		self.connection:Disconnect()
		self.connection = nil
	end
end

function VirtualScroller:refresh()
	local props = self.props
	local state = self.state

	local count = props.count
	local windowSize, canvasPosition = state.WindowSize.Y, state.CanvasPosition.Y
	local bottom = canvasPosition + windowSize

	local minIndex, maxIndex = 1, count
	local padding, canvasSize = 0, 0

	local pos = 0
	for i = 1, count do
		local height = props.getHeightBinding(i):getValue()
		canvasSize += height

		if pos > bottom then
			-- Below window
			if maxIndex > i then
				maxIndex = i
			end
		end

		pos += height

		if pos < canvasPosition then
			-- Above window
			minIndex = i
			padding = pos - height
		end
	end

	self.setPadding(padding)
	self.setTotalCanvas(canvasSize)
	self:setState({
		Start = minIndex,
		End = maxIndex,
	})
end

function VirtualScroller:didUpdate(previousProps)
	if self.props.count ~= previousProps.count then
		-- Items have changed, so we need to refresh
		self:refresh()
	end
end

function VirtualScroller:render()
	local props, state = self.props, self.state

	local items = {}
	for i = state.Start, state.End do
		local content = props.render(i)
		if content == nil then
			continue
		end

		items["Item" .. i] = e("Frame", {
			LayoutOrder = i,
			Size = props.getHeightBinding(i):map(function(height)
				return UDim2.new(1, 0, 0, height)
			end),
			BackgroundTransparency = 1,
		}, content)
	end

	return Theme.with(function(theme)
		return e("ScrollingFrame", {
			Size = props.size,
			Position = props.position,
			AnchorPoint = props.anchorPoint,
			BackgroundTransparency = props.backgroundTransparency or 1,
			BackgroundColor3 = props.backgroundColor3,
			BorderColor3 = props.borderColor3,
			CanvasSize = self.totalCanvas:map(function(s)
				return UDim2.fromOffset(0, s)
			end),
			ScrollBarThickness = 9,
			ScrollBarImageColor3 = theme.ScrollBarColor,
			ScrollBarImageTransparency = props.transparency:map(function(value)
				return bindingUtil.blendAlpha({ 0.65, value })
			end),
			TopImage = Assets.Images.ScrollBar.Top,
			MidImage = Assets.Images.ScrollBar.Middle,
			BottomImage = Assets.Images.ScrollBar.Bottom,

			ElasticBehavior = Enum.ElasticBehavior.Always,
			ScrollingDirection = Enum.ScrollingDirection.Y,
			VerticalScrollBarInset = Enum.ScrollBarInset.ScrollBar,
			[Roact.Ref] = self.scrollFrameRef,
		}, {
			Layout = e("UIListLayout", {
				Padding = UDim.new(0, 0),
				SortOrder = Enum.SortOrder.LayoutOrder,
				FillDirection = Enum.FillDirection.Vertical,
			}),
			Padding = e("UIPadding", {
				PaddingTop = self.padding:map(function(p)
					return UDim.new(0, p)
				end),
			}),
			Content = Roact.createFragment(items),
		})
	end)
end

return VirtualScroller
