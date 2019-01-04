local Roact = require(script:FindFirstAncestor("Rojo").Roact)

local Dictionary = require(script.Parent.Parent.Dictionary)

local e = Roact.createElement

local FitList = Roact.Component:extend("FitList")

function FitList:init()
	self.sizeBinding, self.setSize = Roact.createBinding(UDim2.new())
end

function FitList:render()
	local containerProps = self.props.containerProps
	local layoutProps = self.props.layoutProps
	local paddingProps = self.props.paddingProps

	local padding
	if paddingProps ~= nil then
		padding = e("UIPadding", paddingProps)
	end

	local children = Dictionary.merge(self.props[Roact.Children], {
		["$Layout"] = e("UIListLayout", Dictionary.merge({
			SortOrder = Enum.SortOrder.LayoutOrder,
			[Roact.Change.AbsoluteContentSize] = function(instance)
				local size = instance.AbsoluteContentSize

				if paddingProps ~= nil then
					size = size + Vector2.new(
						paddingProps.PaddingLeft.Offset + paddingProps.PaddingRight.Offset,
						paddingProps.PaddingTop.Offset + paddingProps.PaddingBottom.Offset)
				end

				self.setSize(UDim2.new(0, size.X, 0, size.Y))
			end,
		}, layoutProps)),

		["$Padding"] = padding,
	})

	local fullContainerProps = Dictionary.merge(containerProps, {
		Size = self.sizeBinding,
	})

	return e("Frame", fullContainerProps, children)
end

return FitList