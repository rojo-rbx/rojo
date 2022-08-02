local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin

local Roact = require(Rojo.Roact)
local Log = require(Rojo.Log)

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
		self:setState({	WindowSize = rbx.AbsoluteWindowSize })
		self:refresh()
	end)

	local canvasPositionSignal = rbx:GetPropertyChangedSignal("CanvasPosition")
	self.canvasPositionChanged = canvasPositionSignal:Connect(function()
		if math.abs(rbx.CanvasPosition.Y-self.state.CanvasPosition.Y) >= self.props.baseHeight then
			self:setState({	CanvasPosition = rbx.CanvasPosition })
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

function VirtualScroller:updateRange()
	local props = self.props
	local state = self.state

	local count, baseHeight = props.count, props.baseHeight
	local windowSize, canvasPosition = state.WindowSize.Y, state.CanvasPosition.Y

	local minIndex = 1
	local maxIndex = count

	local bottom = canvasPosition + windowSize

	local pos = 0
	for i=1, count do
		if pos > bottom then
			maxIndex = i
			break
		end

		local height = props.getHeightBinding(i):getValue()
		pos += height + baseHeight

		if pos < canvasPosition then
			minIndex = i
		end
	end

	self:setState({
		Start = minIndex,
		End = maxIndex,
	})

	return minIndex, maxIndex
end

function VirtualScroller:refresh()
	local props = self.props
	local baseHeight = props.baseHeight
	local count = props.count

	local minIndex = self:updateRange()

	local padding = 0
	for i=1, minIndex-1 do
		padding += props.getHeightBinding(i):getValue() + baseHeight
	end

	local canvasHeight = padding
	for i=minIndex, count do
		canvasHeight += props.getHeightBinding(i):getValue() + baseHeight
	end

	self.setPadding(padding)
	self.setTotalCanvas(canvasHeight)

	return padding, canvasHeight
end

function VirtualScroller:render()
	local props, state = self.props, self.state
	local baseHeight = props.baseHeight

	if math.floor(baseHeight) ~= baseHeight then
		Log.debug("VirtualScroller.baseHeight should be an integer or there will be minor accumulated error. " .. debug.traceback())
	end

	local items = {}
	for i = state.Start, state.End do
		items["Item"..i] = e("Frame", {
			LayoutOrder = i,
			Size = props.getHeightBinding(i):map(function(height)
				return UDim2.new(1, 0, 0, height + baseHeight)
			end),
			BackgroundTransparency = 1,
		}, {
			props.render(i)
		})
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
			Padding = e("Frame", {
				LayoutOrder = -math.huge,
				Size = self.padding:map(function(p)
					return UDim2.new(1, 0, 0, p)
				end),
				BackgroundTransparency = 0,
				BackgroundColor3 = Color3.new(1, 0, 0),
			}),
			Content = Roact.createFragment(items),
		})
	end)
end

return VirtualScroller
