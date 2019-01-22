local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin

local Roact = require(Rojo.Roact)

local Config = require(Plugin.Config)
local Version = require(Plugin.Version)
local Assets = require(Plugin.Assets)
local Theme = require(Plugin.Theme)
local joinBindings = require(Plugin.joinBindings)

local FitList = require(Plugin.Components.FitList)
local FitText = require(Plugin.Components.FitText)
local FormButton = require(Plugin.Components.FormButton)
local FormTextInput = require(Plugin.Components.FormTextInput)

local RoundBox = Assets.Slices.RoundBox

local e = Roact.createElement

local ConnectPanel = Roact.Component:extend("ConnectPanel")

function ConnectPanel:init()
	self.footerSize, self.setFooterSize = Roact.createBinding(Vector2.new())
	self.footerVersionSize, self.setFooterVersionSize = Roact.createBinding(Vector2.new())

	-- This is constructed in init because 'joinBindings' is a hack and we'd
	-- leak memory constructing it every render. When this kind of feature lands
	-- in Roact properly, we can do this inline in render without fear.
	self.footerRestSize = joinBindings(
		{
			self.footerSize,
			self.footerVersionSize,
		},
		function(container, other)
			return UDim2.new(0, container.X - other.X - 16, 0, 18)
		end
	)

	self:setState({
		address = Config.defaultHost,
		port = Config.defaultPort,
	})
end

function ConnectPanel:render()
	local startSession = self.props.startSession
	local cancel = self.props.cancel

	return e(FitList, {
		containerKind = "ImageLabel",
		containerProps = {
			Image = RoundBox.asset,
			ImageRectOffset = RoundBox.offset,
			ImageRectSize = RoundBox.size,
			SliceCenter = RoundBox.center,
			ScaleType = Enum.ScaleType.Slice,
			BackgroundTransparency = 1,
			Position = UDim2.new(0.5, 0, 0.5, 0),
			AnchorPoint = Vector2.new(0.5, 0.5),
		},
		layoutProps = {
			HorizontalAlignment = Enum.HorizontalAlignment.Center,
		},
	}, {
		Inputs = e(FitList, {
			containerProps = {
				BackgroundTransparency = 1,
				LayoutOrder = 1,
			},
			layoutProps = {
				FillDirection = Enum.FillDirection.Horizontal,
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
					Padding = UDim.new(0, 4),
				},
			}, {
				Label = e(FitText, {
					MinSize = Vector2.new(0, 20),
					Kind = "TextLabel",
					LayoutOrder = 1,
					BackgroundTransparency = 1,
					TextXAlignment = Enum.TextXAlignment.Left,
					Font = Theme.TitleFont,
					TextSize = 20,
					Text = "Address",
					TextColor3 = Theme.AccentColor,
				}),

				Input = e(FormTextInput, {
					layoutOrder = 2,
					size = UDim2.new(0, 200, 0, 28),
					value = self.state.address,
					onValueChange = function(newValue)
						self:setState({
							address = newValue,
						})
					end,
				}),
			}),

			Port = e(FitList, {
				containerProps = {
					LayoutOrder = 2,
					BackgroundTransparency = 1,
				},
				layoutProps = {
					Padding = UDim.new(0, 4),
				},
			}, {
				Label = e(FitText, {
					MinSize = Vector2.new(0, 20),
					Kind = "TextLabel",
					LayoutOrder = 1,
					BackgroundTransparency = 1,
					TextXAlignment = Enum.TextXAlignment.Left,
					Font = Theme.TitleFont,
					TextSize = 20,
					Text = "Port",
					TextColor3 = Theme.AccentColor,
				}),

				Input = e(FormTextInput, {
					layoutOrder = 2,
					size = UDim2.new(0, 65, 0, 28),
					value = self.state.port,
					onValueChange = function(newValue)
						self:setState({
							port = newValue,
						})
					end,
				}),
			}),
		}),

		Buttons = e(FitList, {
			fitAxes = "Y",
			containerProps = {
				BackgroundTransparency = 1,
				LayoutOrder = 2,
				Size = UDim2.new(1, 0, 0, 0),
			},
			layoutProps = {
				FillDirection = Enum.FillDirection.Horizontal,
				HorizontalAlignment = Enum.HorizontalAlignment.Right,
				Padding = UDim.new(0, 8),
			},
			paddingProps = {
				PaddingTop = UDim.new(0, 0),
				PaddingBottom = UDim.new(0, 8),
				PaddingLeft = UDim.new(0, 8),
				PaddingRight = UDim.new(0, 8),
			},
		}, {
			e(FormButton, {
				layoutOrder = 1,
				text = "Cancel",
				onClick = function()
					if cancel ~= nil then
						cancel()
					end
				end,
				secondary = true,
			}),

			e(FormButton, {
				layoutOrder = 2,
				text = "Connect",
				onClick = function()
					if startSession ~= nil then
						startSession(self.state.address, self.state.port)
					end
				end,
			}),
		}),

		Footer = e(FitList, {
			fitAxes = "Y",
			containerKind = "ImageLabel",
			containerProps = {
				Image = RoundBox.asset,
				ImageRectOffset = RoundBox.offset + Vector2.new(0, RoundBox.size.Y / 2),
				ImageRectSize = RoundBox.size * Vector2.new(1, 0.5),
				SliceCenter = RoundBox.center,
				ScaleType = Enum.ScaleType.Slice,
				ImageColor3 = Theme.SecondaryColor,
				Size = UDim2.new(1, 0, 0, 0),
				LayoutOrder = 3,
				BackgroundTransparency = 1,

				[Roact.Change.AbsoluteSize] = function(rbx)
					self.setFooterSize(rbx.AbsoluteSize)
				end,
			},
			layoutProps = {
				FillDirection = Enum.FillDirection.Horizontal,
				HorizontalAlignment = Enum.HorizontalAlignment.Center,
			},
			paddingProps = {
				PaddingTop = UDim.new(0, 4),
				PaddingBottom = UDim.new(0, 4),
				PaddingLeft = UDim.new(0, 8),
				PaddingRight = UDim.new(0, 8),
			},
		}, {
			LogoContainer = e("Frame", {
				BackgroundTransparency = 1,

				Size = self.footerRestSize,
			}, {
				Logo = e("ImageLabel", {
					Image = Assets.Images.Logo,
					Size = UDim2.new(0, 60, 0, 40),
					ScaleType = Enum.ScaleType.Fit,
					BackgroundTransparency = 1,
					Position = UDim2.new(0, 0, 1, 0),
					AnchorPoint = Vector2.new(0, 1),
				}),
			}),

			Version = e(FitText, {
				Font = Theme.TitleFont,
				TextSize = 18,
				Text = Version.display(Config.version),
				TextXAlignment = Enum.TextXAlignment.Right,
				TextColor3 = Theme.LightTextColor,
				BackgroundTransparency = 1,

				[Roact.Change.AbsoluteSize] = function(rbx)
					self.setFooterVersionSize(rbx.AbsoluteSize)
				end,
			}),
		}),
	})
end

return ConnectPanel