local Roact = require(script:FindFirstAncestor("Rojo").Roact)

local Dictionary = require(script.Parent.Parent.Dictionary)

local e = Roact.createElement

local FitScrollingFrame = Roact.Component:extend("FitScrollingFrame")

function FitScrollingFrame:init()
	self.sizeBinding, self.setSize = Roact.createBinding(UDim2.new())
end

function FitScrollingFrame:render()
	local containerProps = self.props.containerProps
	local layoutProps = self.props.layoutProps

	local children = Dictionary.merge(self.props[Roact.Children], {
		["$Layout"] = e("UIListLayout", Dictionary.merge({
			SortOrder = Enum.SortOrder.LayoutOrder,
			[Roact.Change.AbsoluteContentSize] = function(instance)
				self.setSize(UDim2.new(0, 0, 0, instance.AbsoluteContentSize.Y))
			end,
		}, layoutProps)),
	})

	local fullContainerProps = Dictionary.merge(containerProps, {
		CanvasSize = self.sizeBinding,
	})

	return e("ScrollingFrame", fullContainerProps, children)
end

return FitScrollingFrame