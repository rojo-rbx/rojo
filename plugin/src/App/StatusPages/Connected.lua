local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local Theme = require(Plugin.App.Theme)
local Assets = require(Plugin.Assets)

local Header = require(Plugin.App.Components.Header)
local IconButton = require(Plugin.App.Components.IconButton)
local BorderedContainer = require(Plugin.App.Components.BorderedContainer)
local Tooltip = require(Plugin.App.Components.Tooltip)

local e = Roact.createElement

local AGE_UNITS = { {31556909, "year"}, {2629743, "month"}, {604800, "week"}, {86400, "day"}, {3600, "hour"}, {60, "minute"}, }
function timeSinceText(elapsed: number): string
	if elapsed < 3 then
		return "just now"
	end

	local ageText = string.format("%d seconds ago", elapsed)

	for _,UnitData in ipairs(AGE_UNITS) do
		local UnitSeconds, UnitName = UnitData[1], UnitData[2]
		if elapsed > UnitSeconds then
			local c = math.floor(elapsed/UnitSeconds)
			ageText = string.format("%d %s%s ago", c, UnitName, c>1 and "s" or "")
			break
		end
	end

	return ageText
end

local function ConnectionDetails(props)
	return Theme.with(function(theme)
		return e(BorderedContainer, {
			transparency = props.transparency,
			size = UDim2.new(1, 0, 0, 70),
			layoutOrder = props.layoutOrder,
		}, {
			TextContainer = e("Frame", {
				Size = UDim2.new(1, 0, 1, 0),
				BackgroundTransparency = 1
			}, {
				ProjectName = e("TextLabel", {
					Text = props.projectName,
					Font = Enum.Font.GothamBold,
					TextSize = 20,
					TextColor3 = theme.ConnectionDetails.ProjectNameColor,
					TextTransparency = props.transparency,
					TextXAlignment = Enum.TextXAlignment.Left,

					Size = UDim2.new(1, 0, 0, 20),

					LayoutOrder = 1,
					BackgroundTransparency = 1,
				}),

				Address = e("TextLabel", {
					Text = props.address,
					Font = Enum.Font.Code,
					TextSize = 15,
					TextColor3 = theme.ConnectionDetails.AddressColor,
					TextTransparency = props.transparency,
					TextXAlignment = Enum.TextXAlignment.Left,

					Size = UDim2.new(1, 0, 0, 15),

					LayoutOrder = 2,
					BackgroundTransparency = 1,
				}),

				Layout = e("UIListLayout", {
					VerticalAlignment = Enum.VerticalAlignment.Center,
					FillDirection = Enum.FillDirection.Vertical,
					SortOrder = Enum.SortOrder.LayoutOrder,
					Padding = UDim.new(0, 6),
				}),
			}),

			Disconnect = e(IconButton, {
				icon = Assets.Images.Icons.Close,
				iconSize = 24,
				color = theme.ConnectionDetails.DisconnectColor,
				transparency = props.transparency,

				position = UDim2.new(1, 0, 0.5, 0),
				anchorPoint = Vector2.new(1, 0.5),

				onClick = props.onDisconnect,
			}, {
				Tip = e(Tooltip.Trigger, {
					text = "Disconnect from the Rojo sync server"
				}),
			}),

			Padding = e("UIPadding", {
				PaddingLeft = UDim.new(0, 17),
				PaddingRight = UDim.new(0, 15),
			}),
		})
	end)
end

local ConnectedPage = Roact.Component:extend("ConnectedPage")

function ConnectedPage:render()
	return Theme.with(function(theme)
		return Roact.createFragment({
			Header = e(Header, {
				transparency = self.props.transparency,
				layoutOrder = 1,
			}),

			ConnectionDetails = e(ConnectionDetails, {
				projectName = self.state.projectName,
				address = self.state.address,
				transparency = self.props.transparency,
				layoutOrder = 2,

				onDisconnect = self.props.onDisconnect,
			}),

			Info = e("TextLabel", {
				Text = self.props.patchInfo:map(function(info)
					return string.format(
						"<i>Synced %d change%s %s</i>",
						info.changes,
						info.changes == 1 and "" or "s",
						timeSinceText(os.time() - info.timestamp)
					)
				end),
				Font = Enum.Font.Gotham,
				TextSize = 14,
				TextWrapped = true,
				RichText = true,
				TextColor3 = theme.Header.VersionColor,
				TextXAlignment = Enum.TextXAlignment.Left,
				TextYAlignment = Enum.TextYAlignment.Top,
				TextTransparency = self.props.transparency,

				Size = UDim2.new(1, 0, 0, 28),

				LayoutOrder = 3,
				BackgroundTransparency = 1,
			}),

			Layout = e("UIListLayout", {
				VerticalAlignment = Enum.VerticalAlignment.Center,
				FillDirection = Enum.FillDirection.Vertical,
				SortOrder = Enum.SortOrder.LayoutOrder,
				Padding = UDim.new(0, 10),
			}),

			Padding = e("UIPadding", {
				PaddingLeft = UDim.new(0, 20),
				PaddingRight = UDim.new(0, 20),
			}),
		})
	end)
end

function ConnectedPage.getDerivedStateFromProps(props)
	-- If projectName or address ever get removed from props, make sure we still have
	-- the properties! The component still needs to have its data for it to be properly
	-- animated out without the labels changing.

	return {
		projectName = props.projectName,
		address = props.address,
	}
end

return ConnectedPage
