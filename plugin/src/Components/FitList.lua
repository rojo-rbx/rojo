local Roact = require(script:FindFirstAncestor("Rojo").Roact)

local Dictionary = require(script.Parent.Parent.Dictionary)

local e = Roact.createElement

local FitList = Roact.Component:extend("FitList")

function FitList:init()
	self.sizeBinding, self.setSize = Roact.createBinding(UDim2.new())
end

function FitList:render()
	local containerKind = self.props.containerKind or "Frame"
	local fitAxes = self.props.fitAxes or "XY"
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
				local contentSize = instance.AbsoluteContentSize

				if paddingProps ~= nil then
					contentSize = contentSize + Vector2.new(
						paddingProps.PaddingLeft.Offset + paddingProps.PaddingRight.Offset,
						paddingProps.PaddingTop.Offset + paddingProps.PaddingBottom.Offset)
				end

				local combinedSize

				if fitAxes == "X" then
					combinedSize = UDim2.new(0, contentSize.X, containerProps.Size.Y.Scale, containerProps.Size.Y.Offset)
				elseif fitAxes == "Y" then
					combinedSize = UDim2.new(containerProps.Size.X.Scale, containerProps.Size.X.Offset, 0, contentSize.Y)
				elseif fitAxes == "XY" then
					combinedSize = UDim2.new(0, contentSize.X, 0, contentSize.Y)
				else
					error("Invalid fitAxes value")
				end

				self.setSize(combinedSize)
			end,
		}, layoutProps)),

		["$Padding"] = padding,
	})

	local fullContainerProps = Dictionary.merge(containerProps, {
		Size = self.sizeBinding,
	})

	return e(containerKind, fullContainerProps, children)
end

return FitList