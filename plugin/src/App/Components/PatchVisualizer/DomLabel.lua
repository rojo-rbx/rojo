local SelectionService = game:GetService("Selection")
local StudioService = game:GetService("StudioService")

local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)
local Flipper = require(Packages.Flipper)

local Assets = require(Plugin.Assets)
local Theme = require(Plugin.App.Theme)
local bindingUtil = require(Plugin.App.bindingUtil)

local e = Roact.createElement

local ChangeList = require(script.Parent.ChangeList)
local Tooltip = require(script.Parent.Parent.Tooltip)

local function getRecoloredClassIcon(className, color)
	local iconProps = StudioService:GetClassIcon(className)

	if iconProps then
		local success, editableImageSize, editableImagePixels = pcall(function()
			local editableImage = game:GetService("AssetService"):CreateEditableImageAsync(iconProps.Image)
			local pixels = editableImage:ReadPixels(Vector2.zero, editableImage.Size)
			local hue, sat = color:ToHSV()
			for i = 1, #pixels, 4 do
				if pixels[i + 3] == 0 then
					continue
				end
				local pixelColor = Color3.new(pixels[i], pixels[i + 1], pixels[i + 2])
				local _, _, val = pixelColor:ToHSV()
				local newPixelColor = Color3.fromHSV(hue, sat, val)
				pixels[i], pixels[i + 1], pixels[i + 2] = newPixelColor.R, newPixelColor.G, newPixelColor.B
			end
			return editableImage.Size, pixels
		end)
		if success then
			iconProps.EditableImagePixels = editableImagePixels
			iconProps.EditableImageSize = editableImageSize
		end
	end

	return iconProps
end

local EditableImage = Roact.Component:extend("EditableImage")

function EditableImage:init()
	self.ref = Roact.createRef()
end

function EditableImage:writePixels()
	local image = self.ref.current
	if not image then
		return
	end
	if not self.props.pixels then
		return
	end

	image:WritePixels(Vector2.zero, self.props.size, self.props.pixels)
end

function EditableImage:render()
	return e("EditableImage", {
		Size = self.props.size,
		[Roact.Ref] = self.ref,
	})
end

function EditableImage:didMount()
	self:writePixels()
end

function EditableImage:didUpdate()
	self:writePixels()
end

local Expansion = Roact.Component:extend("Expansion")

function Expansion:render()
	local props = self.props

	if not props.rendered then
		return nil
	end

	return e("Frame", {
		BackgroundTransparency = 1,
		Size = UDim2.new(1, -props.indent, 1, -24),
		Position = UDim2.new(0, props.indent, 0, 24),
	}, {
		ChangeList = e(ChangeList, {
			changes = props.changeList,
			transparency = props.transparency,
			showStringDiff = props.showStringDiff,
			showTableDiff = props.showTableDiff,
		}),
	})
end

local DomLabel = Roact.Component:extend("DomLabel")

function DomLabel:init()
	local initHeight = self.props.elementHeight:getValue()
	self.expanded = initHeight > 24

	self.motor = Flipper.SingleMotor.new(initHeight)
	self.binding = bindingUtil.fromMotor(self.motor)

	self:setState({
		renderExpansion = self.expanded,
	})
	self.motor:onStep(function(value)
		local renderExpansion = value > 24

		self.props.setElementHeight(value)
		if self.props.updateEvent then
			self.props.updateEvent:Fire()
		end

		self:setState(function(state)
			if state.renderExpansion == renderExpansion then
				return nil
			end

			return {
				renderExpansion = renderExpansion,
			}
		end)
	end)
end

function DomLabel:didUpdate(prevProps)
	if
		prevProps.instance ~= self.props.instance
		or prevProps.patchType ~= self.props.patchType
		or prevProps.name ~= self.props.name
		or prevProps.changeList ~= self.props.changeList
	then
		-- Close the expansion when the domlabel is changed to a different thing
		self.expanded = false
		self.motor:setGoal(Flipper.Spring.new(24, {
			frequency = 5,
			dampingRatio = 1,
		}))
	end
end

function DomLabel:render()
	local props = self.props
	local depth = props.depth or 0

	return Theme.with(function(theme)
		local color = if props.isWarning
			then theme.Diff.Warning
			elseif props.patchType then theme.Diff[props.patchType]
			else theme.TextColor
		local iconProps = getRecoloredClassIcon(props.className, color)

		local indent = (props.depth or 0) * 12 + 15

		-- Line guides help indent depth remain readable
		local lineGuides = {}
		for i = 1, depth do
			if i == depth and props.isFinalChild and not props.hasChildren then
				lineGuides["Line_" .. i] = e("Frame", {
					Size = UDim2.new(0, 2, 1, 2 - 12),
					Position = UDim2.new(0, (12 * i) + 6, 0, -1),
					BorderSizePixel = 0,
					BackgroundTransparency = props.transparency,
					BackgroundColor3 = theme.BorderedContainer.BorderColor,
				})
			else
				lineGuides["Line_" .. i] = e("Frame", {
					Size = UDim2.new(0, 2, 1, 2),
					Position = UDim2.new(0, (12 * i) + 6, 0, -1),
					BorderSizePixel = 0,
					BackgroundTransparency = props.transparency,
					BackgroundColor3 = theme.BorderedContainer.BorderColor,
				})
			end
		end

		lineGuides["Connector"] = e("Frame", {
			Size = UDim2.new(0, 8, 0, 2),
			Position = UDim2.new(0, (12 * props.depth) + 6, 0, 12),
			BorderSizePixel = 0,
			BackgroundTransparency = props.transparency,
			BackgroundColor3 = theme.BorderedContainer.BorderColor,
		})

		return e("Frame", {
			ClipsDescendants = true,
			BackgroundTransparency = if props.elementIndex % 2 == 0 then 0.98 else 1,
			BackgroundColor3 = theme.Diff.Row,
			Size = self.binding:map(function(expand)
				return UDim2.new(1, 0, 0, expand)
			end),
		}, {
			Padding = e("UIPadding", {
				PaddingLeft = UDim.new(0, 10),
				PaddingRight = UDim.new(0, 10),
			}),
			Button = e("TextButton", {
				BackgroundTransparency = 1,
				Text = "",
				Size = UDim2.new(1, 0, 1, 0),
				[Roact.Event.Activated] = function(_rbx: Instance, _input: InputObject, clickCount: number)
					if clickCount == 1 then
						-- Double click opens the instance in explorer
						self.lastDoubleClickTime = os.clock()
						if props.instance then
							SelectionService:Set({ props.instance })
						end
					elseif clickCount == 0 then
						-- Single click expands the changes
						task.wait(0.25)
						if os.clock() - (self.lastDoubleClickTime or 0) <= 0.25 then
							-- This is a double click, so don't expand
							return
						end

						if props.changeList then
							self.expanded = not self.expanded
							local goalHeight = 24
								+ (if self.expanded then math.clamp(#props.changeList * 24, 24, 24 * 6) else 0)
							self.motor:setGoal(Flipper.Spring.new(goalHeight, {
								frequency = 5,
								dampingRatio = 1,
							}))
						end
					end
				end,
			}, {
				StateTip = if (props.instance or props.changeList)
					then e(Tooltip.Trigger, {
						text = (if props.changeList
							then "Click to " .. (if self.expanded then "hide" else "view") .. " changes"
							else "") .. (if props.instance
							then (if props.changeList then " & d" else "D") .. "ouble click to open in Explorer"
							else ""),
					})
					else nil,
			}),
			Expansion = if props.changeList
				then e(Expansion, {
					rendered = self.state.renderExpansion,
					indent = indent,
					transparency = props.transparency,
					changeList = props.changeList,
					showStringDiff = props.showStringDiff,
					showTableDiff = props.showTableDiff,
				})
				else nil,
			DiffIcon = if props.patchType
				then e("ImageLabel", {
					Image = Assets.Images.Diff[props.patchType],
					ImageColor3 = color,
					ImageTransparency = props.transparency,
					BackgroundTransparency = 1,
					Size = UDim2.new(0, 14, 0, 14),
					Position = UDim2.new(0, 0, 0, 12),
					AnchorPoint = Vector2.new(0, 0.5),
				})
				else nil,
			ClassIcon = e(
				"ImageLabel",
				{
					Image = iconProps.Image,
					ImageTransparency = props.transparency,
					ImageRectOffset = iconProps.ImageRectOffset,
					ImageRectSize = iconProps.ImageRectSize,
					BackgroundTransparency = 1,
					Size = UDim2.new(0, 16, 0, 16),
					Position = UDim2.new(0, indent + 2, 0, 12),
					AnchorPoint = Vector2.new(0, 0.5),
				},
				if iconProps.EditableImagePixels
					then e(EditableImage, {
						size = iconProps.EditableImageSize,
						pixels = iconProps.EditableImagePixels, --
					})
					else nil
			),
			InstanceName = e("TextLabel", {
				Text = (if props.isWarning then "⚠ " else "") .. props.name,
				RichText = true,
				BackgroundTransparency = 1,
				Font = if props.patchType then Enum.Font.GothamBold else Enum.Font.GothamMedium,
				TextSize = 14,
				TextColor3 = color,
				TextXAlignment = Enum.TextXAlignment.Left,
				TextTransparency = props.transparency,
				TextTruncate = Enum.TextTruncate.AtEnd,
				Size = UDim2.new(1, -indent - 50, 0, 24),
				Position = UDim2.new(0, indent + 22, 0, 0),
			}),
			LineGuides = e("Folder", nil, lineGuides),
		})
	end)
end

return DomLabel
