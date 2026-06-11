local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local Assets = require(Plugin.Assets)
local Theme = require(Plugin.App.Theme)

local getThirdPartyIcon = require(Plugin.getThirdPartyIcon)

local ScrollingFrame = require(Plugin.App.Components.ScrollingFrame)
local TextButton = require(Plugin.App.Components.TextButton)

local e = Roact.createElement

local DIVIDER_FADE_SIZE = 0.1

local PermissionPopup = Roact.Component:extend("PermissionPopup")

function PermissionPopup:init()
	self.contentSize, self.setContentSize = Roact.createBinding(Vector2.new(0, 0))
	self.infoSize, self.setInfoSize = Roact.createBinding(Vector2.new(0, 0))
end

function PermissionPopup:render()
	return Theme.with(function(theme)
		local settingsTheme = theme.Settings

		local iconAsset = getThirdPartyIcon(self.props.source)

		local apiRequests = {
			Event = {},
			Property = {},
			Method = {},
		}
		for index, api in self.props.apis do
			local apiDesc = self.props.apiDescriptions[api]

			apiRequests[apiDesc.Type][api] = e("Frame", {
				LayoutOrder = index,
				BackgroundTransparency = 1,
				Size = UDim2.new(1, 0, 0, 17),
				AutomaticSize = Enum.AutomaticSize.Y,
			}, {
				Divider = e("Frame", {
					BackgroundColor3 = settingsTheme.DividerColor,
					BackgroundTransparency = self.props.transparency,
					Size = UDim2.new(1, 0, 0, 1),
					Position = UDim2.new(0, 0, 0, -2),
					BorderSizePixel = 0,
				}, {
					Gradient = e("UIGradient", {
						Transparency = NumberSequence.new({
							NumberSequenceKeypoint.new(0, 1),
							NumberSequenceKeypoint.new(DIVIDER_FADE_SIZE, 0),
							NumberSequenceKeypoint.new(1 - DIVIDER_FADE_SIZE, 0),
							NumberSequenceKeypoint.new(1, 1),
						}),
					}),
				}),
				Name = e("TextLabel", {
					BackgroundTransparency = 1,
					Position = UDim2.new(0, 0, 0, 0),
					Size = UDim2.new(0, 140, 0, 17),
					TextWrapped = true,
					AutomaticSize = Enum.AutomaticSize.Y,
					Text = api,
					FontFace = theme.Font.Thin,
					TextSize = theme.TextSize.Medium,
					TextColor3 = settingsTheme.Setting.NameColor,
					TextXAlignment = Enum.TextXAlignment.Left,
					TextTransparency = self.props.transparency,
				}),
				Desc = e("TextLabel", {
					BackgroundTransparency = 1,
					Position = UDim2.new(0, 145, 0, 0),
					Size = UDim2.new(1, -145, 0, 17),
					TextWrapped = true,
					AutomaticSize = Enum.AutomaticSize.Y,
					Text = apiDesc.Description,
					FontFace = theme.Font.Thin,
					TextSize = theme.TextSize.Body,
					TextColor3 = settingsTheme.Setting.DescriptionColor,
					TextXAlignment = Enum.TextXAlignment.Left,
					TextTransparency = self.props.transparency,
				}),
			})
		end

		-- Add labels to explain the api types
		if next(apiRequests.Event) then
			apiRequests.Event["_apiTypeInfo"] = e("TextLabel", {
				LayoutOrder = -1,
				BackgroundTransparency = 1,
				Size = UDim2.new(1, 0, 0, 18),
				Text = string.format("%s will be able to listen to these events:", self.props.name),
				TextWrapped = true,
				AutomaticSize = Enum.AutomaticSize.Y,
				FontFace = theme.Font.Main,
				TextSize = theme.TextSize.Medium,
				TextColor3 = settingsTheme.Setting.NameColor,
				TextXAlignment = Enum.TextXAlignment.Left,
				TextTransparency = self.props.transparency,
			}, e("UIPadding", { PaddingBottom = UDim.new(0, 8) }))
		end
		if next(apiRequests.Property) then
			apiRequests.Property["_apiTypeInfo"] = e("TextLabel", {
				LayoutOrder = -1,
				BackgroundTransparency = 1,
				Size = UDim2.new(1, 0, 0, 18),
				Text = string.format("%s will be able to read these properties:", self.props.name),
				TextWrapped = true,
				AutomaticSize = Enum.AutomaticSize.Y,
				FontFace = theme.Font.Main,
				TextSize = theme.TextSize.Medium,
				TextColor3 = settingsTheme.Setting.NameColor,
				TextXAlignment = Enum.TextXAlignment.Left,
				TextTransparency = self.props.transparency,
			}, e("UIPadding", { PaddingBottom = UDim.new(0, 8) }))
		end
		if next(apiRequests.Method) then
			apiRequests.Method["_apiTypeInfo"] = e("TextLabel", {
				LayoutOrder = -1,
				BackgroundTransparency = 1,
				Size = UDim2.new(1, 0, 0, 18),
				Text = string.format("%s will be able to call these methods:", self.props.name),
				TextWrapped = true,
				AutomaticSize = Enum.AutomaticSize.Y,
				FontFace = theme.Font.Main,
				TextSize = theme.TextSize.Medium,
				TextColor3 = settingsTheme.Setting.NameColor,
				TextXAlignment = Enum.TextXAlignment.Left,
				TextTransparency = self.props.transparency,
			}, e("UIPadding", { PaddingBottom = UDim.new(0, 8) }))
		end

		return e("Frame", {
			BackgroundTransparency = 1,
			Size = UDim2.new(1, 0, 1, 0),
		}, {
			Layout = e("UIListLayout", {
				FillDirection = Enum.FillDirection.Vertical,
				SortOrder = Enum.SortOrder.LayoutOrder,
				Padding = UDim.new(0, 15),
				HorizontalAlignment = Enum.HorizontalAlignment.Center,
			}),

			Padding = e("UIPadding", {
				PaddingLeft = UDim.new(0, 20),
				PaddingRight = UDim.new(0, 20),
				PaddingTop = UDim.new(0, 15),
				PaddingBottom = UDim.new(0, 15),
			}),

			Icons = e("Frame", {
				BackgroundTransparency = 1,
				Size = UDim2.new(1, 0, 0, 32),
				LayoutOrder = 1,
			}, {
				Layout = e("UIListLayout", {
					FillDirection = Enum.FillDirection.Horizontal,
					SortOrder = Enum.SortOrder.LayoutOrder,
					Padding = UDim.new(0, 5),
					HorizontalAlignment = Enum.HorizontalAlignment.Center,
					VerticalAlignment = Enum.VerticalAlignment.Center,
				}),

				ThirdPartyIcon = e("ImageLabel", {
					BackgroundTransparency = 1,
					Size = UDim2.new(0, 32, 0, 32),
					Image = iconAsset,
					LayoutOrder = 1,
				}),

				TransactIcon = e("ImageLabel", {
					BackgroundTransparency = 1,
					Size = UDim2.new(0, 24, 0, 24),
					Image = Assets.Images.Icons.Transact,
					ImageColor3 = settingsTheme.Setting.DescriptionColor,
					LayoutOrder = 2,
				}),

				RojoIcon = e("ImageLabel", {
					BackgroundTransparency = 1,
					Size = UDim2.new(0, 32, 0, 32),
					Image = Assets.Images.PluginButton,
					LayoutOrder = 3,
				}),
			}),

			Info = e("TextLabel", {
				BackgroundTransparency = 1,
				Size = UDim2.new(1, 0, 0, 0),
				AutomaticSize = Enum.AutomaticSize.Y,
				Text = string.format("%s is asking to use the Rojo API", self.props.name or "[Unknown]"),
				FontFace = theme.Font.Bold,
				TextSize = theme.TextSize.Medium,
				TextColor3 = settingsTheme.Setting.NameColor,
				TextXAlignment = Enum.TextXAlignment.Center,
				TextWrapped = true,
				TextTransparency = self.props.transparency,
				LayoutOrder = 2,

				[Roact.Change.AbsoluteSize] = function(rbx)
					self.setInfoSize(rbx.AbsoluteSize)
				end,
			}),

			Divider = e("Frame", {
				LayoutOrder = 3,
				BackgroundColor3 = settingsTheme.DividerColor,
				BackgroundTransparency = self.props.transparency,
				Size = UDim2.new(1, 0, 0, 1),
				Position = UDim2.new(0, 0, 0, -2),
				BorderSizePixel = 0,
			}, {
				Gradient = e("UIGradient", {
					Transparency = NumberSequence.new({
						NumberSequenceKeypoint.new(0, 1),
						NumberSequenceKeypoint.new(DIVIDER_FADE_SIZE, 0),
						NumberSequenceKeypoint.new(1 - DIVIDER_FADE_SIZE, 0),
						NumberSequenceKeypoint.new(1, 1),
					}),
				}),
			}),

			ScrollingFrame = e(ScrollingFrame, {
				size = self.infoSize:map(function(infoSize)
					return UDim2.new(0.9, 0, 1, -infoSize.Y - 140)
				end),
				layoutOrder = 9,
				contentSize = self.contentSize,
				transparency = self.props.transparency,
			}, {
				Layout = e("UIListLayout", {
					FillDirection = Enum.FillDirection.Vertical,
					SortOrder = Enum.SortOrder.LayoutOrder,
					Padding = UDim.new(0, 18),

					[Roact.Change.AbsoluteContentSize] = function(object)
						self.setContentSize(object.AbsoluteContentSize)
					end,
				}),

				PropertyRequests = e("Frame", {
					BackgroundTransparency = 1,
					Size = UDim2.new(1, 0, 0, 0),
					AutomaticSize = Enum.AutomaticSize.Y,
					LayoutOrder = 1,
				}, {
					APIs = Roact.createFragment(apiRequests.Property),
					Layout = e("UIListLayout", {
						FillDirection = Enum.FillDirection.Vertical,
						SortOrder = Enum.SortOrder.LayoutOrder,
						Padding = UDim.new(0, 4),
					}),
				}),

				EventRequests = e("Frame", {
					BackgroundTransparency = 1,
					Size = UDim2.new(1, 0, 0, 0),
					AutomaticSize = Enum.AutomaticSize.Y,
					LayoutOrder = 2,
				}, {
					APIs = Roact.createFragment(apiRequests.Event),
					Layout = e("UIListLayout", {
						FillDirection = Enum.FillDirection.Vertical,
						SortOrder = Enum.SortOrder.LayoutOrder,
						Padding = UDim.new(0, 4),
					}),
				}),

				MethodRequests = e("Frame", {
					BackgroundTransparency = 1,
					Size = UDim2.new(1, 0, 0, 0),
					AutomaticSize = Enum.AutomaticSize.Y,
					LayoutOrder = 3,
				}, {
					APIs = Roact.createFragment(apiRequests.Method),
					Layout = e("UIListLayout", {
						FillDirection = Enum.FillDirection.Vertical,
						SortOrder = Enum.SortOrder.LayoutOrder,
						Padding = UDim.new(0, 4),
					}),
				}),
			}),

			Actions = e("Frame", {
				BackgroundTransparency = 1,
				Size = UDim2.new(1, 0, 0, 34),
				LayoutOrder = 10,
			}, {
				Layout = e("UIListLayout", {
					FillDirection = Enum.FillDirection.Horizontal,
					HorizontalAlignment = Enum.HorizontalAlignment.Center,
					VerticalAlignment = Enum.VerticalAlignment.Center,
					SortOrder = Enum.SortOrder.LayoutOrder,
					Padding = UDim.new(0, 10),
				}),

				Deny = e(TextButton, {
					text = "Deny",
					style = "Bordered",
					transparency = self.props.transparency,
					layoutOrder = 1,
					onClick = function()
						self.props.responseEvent:Fire(false)
					end,
				}),

				Allow = e(TextButton, {
					text = "Allow",
					style = "Solid",
					transparency = self.props.transparency,
					layoutOrder = 2,
					onClick = function()
						self.props.responseEvent:Fire(true)
					end,
				}),
			}),
		})
	end)
end

return PermissionPopup
