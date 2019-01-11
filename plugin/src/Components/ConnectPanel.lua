local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin

local Roact = require(Rojo.Roact)

local Config = require(Plugin.Config)
local Assets = require(Plugin.Assets)

local FitList = require(Plugin.Components.FitList)
local FitText = require(Plugin.Components.FitText)
local FormButton = require(Plugin.Components.FormButton)

local WhiteCross = Assets.Sprites.WhiteCross
local GrayBox = Assets.Slices.GrayBox

local e = Roact.createElement

local TEXT_COLOR = Color3.new(0.05, 0.05, 0.05)
local FORM_TEXT_SIZE = 20

local ConnectPanel = Roact.Component:extend("ConnectPanel")

function ConnectPanel:init()
	self.labelSizes = {}
	self.labelSize, self.setLabelSize = Roact.createBinding(Vector2.new())

	self:setState({
		address = Config.defaultHost,
		port = Config.defaultPort,
	})
end

function ConnectPanel:updateLabelSize(name, size)
	self.labelSizes[name] = size

	local x = 0
	local y = 0

	for _, size in pairs(self.labelSizes) do
		x = math.max(x, size.X)
		y = math.max(y, size.Y)
	end

	self.setLabelSize(Vector2.new(x, y))
end

function ConnectPanel:render()
	local startSession = self.props.startSession
	local cancel = self.props.cancel

	return e(FitList, {
		containerKind = "ImageLabel",
		containerProps = {
			Image = GrayBox.asset,
			ImageRectOffset = GrayBox.offset,
			ImageRectSize = GrayBox.size,
			ScaleType = Enum.ScaleType.Slice,
			SliceCenter = GrayBox.center,
			BackgroundTransparency = 1,
			Position = UDim2.new(0.5, 0, 0.5, 0),
			AnchorPoint = Vector2.new(0.5, 0.5),
		},
		layoutProps = {
			HorizontalAlignment = Enum.HorizontalAlignment.Center,
		},
	}, {
		Head = e("Frame", {
			LayoutOrder = 1,
			Size = UDim2.new(1, 0, 0, 36),
			BackgroundTransparency = 1,
		}, {
			Padding = e("UIPadding", {
				PaddingTop = UDim.new(0, 8),
				PaddingBottom = UDim.new(0, 8),
				PaddingLeft = UDim.new(0, 8),
				PaddingRight = UDim.new(0, 8),
			}),

			Title = e("TextLabel", {
				Font = Enum.Font.SourceSansBold,
				TextSize = 22,
				Text = "Start New Rojo Session",
				Size = UDim2.new(1, 0, 1, 0),
				TextXAlignment = Enum.TextXAlignment.Left,
				BackgroundTransparency = 1,
				TextColor3 = TEXT_COLOR,
			}),

			Close = e("ImageButton", {
				Image = WhiteCross.asset,
				ImageRectOffset = WhiteCross.offset,
				ImageRectSize = WhiteCross.size,
				Size = UDim2.new(0, 18, 0, 18),
				Position = UDim2.new(1, 0, 0.5, 0),
				AnchorPoint = Vector2.new(1, 0.5),
				ImageColor3 = TEXT_COLOR,
				BackgroundTransparency = 1,
				[Roact.Event.Activated] = function()
					cancel()
				end,
			}),
		}),

		Border = e("Frame", {
			BorderSizePixel = 0,
			BackgroundColor3 = Color3.new(0.7, 0.7, 0.7),
			Size = UDim2.new(1, -4, 0, 2),
			LayoutOrder = 2,
		}),

		Body = e(FitList, {
			containerProps = {
				BackgroundTransparency = 1,
				LayoutOrder = 3,
			},
			layoutProps = {
				Padding = UDim.new(0, 8),
			},
			paddingProps = {
				PaddingTop = UDim.new(0, 8),
				PaddingBottom = UDim.new(0, 8),
				PaddingLeft = UDim.new(0, 8),
				PaddingRight = UDim.new(0, 8),
			},
		}, {
			Address = e(FitList, {
				containerProps = {
					LayoutOrder = 1,
					BackgroundTransparency = 1,
				},
				layoutProps = {
					FillDirection = Enum.FillDirection.Horizontal,
					Padding = UDim.new(0, 8),
				},
			}, {
				Label = e(FitText, {
					MinSize = Vector2.new(0, 24),
					Kind = "TextLabel",
					LayoutOrder = 1,
					BackgroundTransparency = 1,
					TextXAlignment = Enum.TextXAlignment.Left,
					Font = Enum.Font.SourceSansBold,
					TextSize = FORM_TEXT_SIZE,
					Text = "Address",
					TextColor3 = TEXT_COLOR,

					[Roact.Change.AbsoluteSize] = function(rbx)
						self:updateLabelSize("address", rbx.AbsoluteSize)
					end,
				}, {
					Sizing = e("UISizeConstraint", {
						MinSize = self.labelSize,
					}),
				}),

				InputOuter = e("ImageLabel", {
					LayoutOrder = 2,
					Image = GrayBox.asset,
					ImageRectOffset = GrayBox.offset,
					ImageRectSize = GrayBox.size,
					ScaleType = Enum.ScaleType.Slice,
					SliceCenter = GrayBox.center,
					Size = UDim2.new(0, 300, 0, 24),
					BackgroundTransparency = 1,
				}, {
					InputInner = e("TextBox", {
						BackgroundTransparency = 1,
						Size = UDim2.new(1, -8, 1, -8),
						Position = UDim2.new(0.5, 0, 0.5, 0),
						AnchorPoint = Vector2.new(0.5, 0.5),
						Font = Enum.Font.SourceSans,
						ClearTextOnFocus = false,
						TextXAlignment = Enum.TextXAlignment.Left,
						TextSize = FORM_TEXT_SIZE,
						Text = self.state.address,
						TextColor3 = TEXT_COLOR,

						[Roact.Change.Text] = function(rbx)
							self:setState({
								address = rbx.Text,
							})
						end,
					}),
				}),
			}),

			Port = e(FitList, {
				containerProps = {
					LayoutOrder = 2,
					BackgroundTransparency = 1,
				},
				layoutProps = {
					FillDirection = Enum.FillDirection.Horizontal,
					Padding = UDim.new(0, 8),
				},
			}, {
				Label = e(FitText, {
					MinSize = Vector2.new(0, 24),
					Kind = "TextLabel",
					LayoutOrder = 1,
					BackgroundTransparency = 1,
					TextXAlignment = Enum.TextXAlignment.Left,
					Font = Enum.Font.SourceSansBold,
					TextSize = FORM_TEXT_SIZE,
					Text = "Port",
					TextColor3 = TEXT_COLOR,

					[Roact.Change.AbsoluteSize] = function(rbx)
						self:updateLabelSize("port", rbx.AbsoluteSize)
					end,
				}, {
					Sizing = e("UISizeConstraint", {
						MinSize = self.labelSize,
					}),
				}),

				InputOuter = e("ImageLabel", {
					LayoutOrder = 2,
					Image = GrayBox.asset,
					ImageRectOffset = GrayBox.offset,
					ImageRectSize = GrayBox.size,
					ScaleType = Enum.ScaleType.Slice,
					SliceCenter = GrayBox.center,
					Size = UDim2.new(0, 300, 0, 24),
					BackgroundTransparency = 1,
				}, {
					InputInner = e("TextBox", {
						BackgroundTransparency = 1,
						Size = UDim2.new(1, -8, 1, -8),
						Position = UDim2.new(0.5, 0, 0.5, 0),
						AnchorPoint = Vector2.new(0.5, 0.5),
						Font = Enum.Font.SourceSans,
						ClearTextOnFocus = false,
						TextXAlignment = Enum.TextXAlignment.Left,
						TextSize = FORM_TEXT_SIZE,
						Text = self.state.port,
						TextColor3 = TEXT_COLOR,

						[Roact.Change.Text] = function(rbx)
							self:setState({
								port = rbx.Text,
							})
						end,
					}),
				}),
			}),

			Buttons = e(FitList, {
				containerProps = {
					LayoutOrder = 3,
					BackgroundTransparency = 1,
				},
				layoutProps = {
					FillDirection = Enum.FillDirection.Horizontal,
					Padding = UDim.new(0, 8),
				},
			}, {
				e(FormButton, {
					text = "Start",
					onClick = function()
						if startSession ~= nil then
							startSession(self.state.address, self.state.port)
						end
					end,
				}),

				e(FormButton, {
					text = "Cancel",
					onClick = function()
						if cancel ~= nil then
							cancel()
						end
					end,
				}),
			})
		})
	})
end

return ConnectPanel