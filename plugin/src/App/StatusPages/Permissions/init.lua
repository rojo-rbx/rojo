local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local Assets = require(Plugin.Assets)
local Theme = require(Plugin.App.Theme)
local getThirdPartyIcon = require(Plugin.getThirdPartyIcon)

local IconButton = require(Plugin.App.Components.IconButton)
local ScrollingFrame = require(Plugin.App.Components.ScrollingFrame)
local Tooltip = require(Plugin.App.Components.Tooltip)
local SourceListing = require(script.SourceListing)

local e = Roact.createElement

local function Navbar(props)
	return Theme.with(function(theme)
		local navbarTheme = theme.Settings.Navbar

		return e("Frame", {
			Size = UDim2.new(1, 0, 0, 46),
			LayoutOrder = props.layoutOrder,
			BackgroundTransparency = 1,
		}, {
			Padding = e("UIPadding", {
				PaddingLeft = UDim.new(0, 20),
				PaddingRight = UDim.new(0, 20),
			}),

			Back = e(IconButton, {
				icon = Assets.Images.Icons.Back,
				iconSize = 24,
				color = navbarTheme.BackButtonColor,
				transparency = props.transparency,

				position = UDim2.new(0, 0, 0.5, 0),
				anchorPoint = Vector2.new(0, 0.5),

				onClick = props.onBack,
			}, {
				Tip = e(Tooltip.Trigger, {
					text = "Back",
				}),
			}),

			Text = e("TextLabel", {
				Text = "Permissions",
				FontFace = theme.Font.Thin,
				TextSize = theme.TextSize.Large,
				TextColor3 = navbarTheme.TextColor,
				TextTransparency = props.transparency,

				Size = UDim2.new(1, 0, 1, 0),

				BackgroundTransparency = 1,
			}),
		})
	end)
end

local PermissionsPage = Roact.Component:extend("PermissionsPage")

function PermissionsPage:init()
	self.contentSize, self.setContentSize = Roact.createBinding(Vector2.new(0, 0))

	self:setState({
		permissions = self.props.headlessAPI._permissions,
	})

	self.changedListener = self.props.headlessAPI._permissionsChanged:Connect(function()
		self:setState({
			permissions = self.props.headlessAPI._permissions,
		})
	end)
end

function PermissionsPage:willUnmount()
	self.changedListener:Disconnect()
end

function PermissionsPage:render()
	return Theme.with(function(theme)
		local settingsTheme = theme.Settings

		local sources = {}
		for source, permissions in self.state.permissions do
			if next(permissions) == nil then
				continue
			end

			local callerInfoFromSource = self.props.headlessAPI:_getCallerInfoFromSource(source)
			sources[source] = e(SourceListing, {
				layoutOrder = string.byte(source),
				transparency = self.props.transparency,

				callerInfoFromSource = callerInfoFromSource,

				icon = getThirdPartyIcon(source),

				onClick = function()
					self.props.onEdit(
						self.props.headlessAPI._sourceToPlugin[source],
						source,
						callerInfoFromSource,
						self.props.headlessAPI._permissions[source] or {}
					)
				end,
			})
		end

		if next(sources) == nil then
			sources.noSources = e("TextLabel", {
				Text = "No third-party plugins have been granted permissions.",
				FontFace = theme.Font.Thin,
				TextSize = theme.TextSize.Large,
				TextColor3 = settingsTheme.Setting.DescriptionColor,
				TextTransparency = self.props.transparency,
				TextWrapped = true,

				Size = UDim2.new(1, 0, 0, 48),
				LayoutOrder = 0,

				BackgroundTransparency = 1,
			})
		end

		return e("Frame", {
			Size = UDim2.new(1, 0, 1, 0),
			BackgroundTransparency = 1,
		}, {
			Navbar = e(Navbar, {
				onBack = self.props.onBack,
				transparency = self.props.transparency,
			}),

			PluginSources = e(ScrollingFrame, {
				size = UDim2.new(1, 0, 1, -47),
				position = UDim2.new(0, 0, 0, 47),
				contentSize = self.contentSize,
				transparency = self.props.transparency,
			}, {
				Layout = e("UIListLayout", {
					FillDirection = Enum.FillDirection.Vertical,
					SortOrder = Enum.SortOrder.LayoutOrder,

					[Roact.Change.AbsoluteContentSize] = function(object)
						self.setContentSize(object.AbsoluteContentSize)
					end,
				}),

				Padding = e("UIPadding", {
					PaddingLeft = UDim.new(0, 20),
					PaddingRight = UDim.new(0, 20),
				}),

				Sources = Roact.createFragment(sources),
			}),
		})
	end)
end

return PermissionsPage
