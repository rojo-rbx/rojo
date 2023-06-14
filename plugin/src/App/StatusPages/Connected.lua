local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)
local Flipper = require(Packages.Flipper)

local bindingUtil = require(Plugin.App.bindingUtil)
local Theme = require(Plugin.App.Theme)
local Assets = require(Plugin.Assets)
local PatchSet = require(Plugin.PatchSet)

local Header = require(Plugin.App.Components.Header)
local IconButton = require(Plugin.App.Components.IconButton)
local BorderedContainer = require(Plugin.App.Components.BorderedContainer)
local Tooltip = require(Plugin.App.Components.Tooltip)
local PatchVisualizer = require(Plugin.App.Components.PatchVisualizer)

local e = Roact.createElement

local AGE_UNITS = { {31556909, "year"}, {2629743, "month"}, {604800, "week"}, {86400, "day"}, {3600, "hour"}, {60, "minute"}, }
function timeSinceText(elapsed: number): string
	if elapsed < 3 then
		return "just now"
	end

	local ageText = string.format("%d seconds ago", elapsed)

	for _, UnitData in ipairs(AGE_UNITS) do
		local UnitSeconds, UnitName = UnitData[1], UnitData[2]
		if elapsed > UnitSeconds then
			local c = math.floor(elapsed / UnitSeconds)
			ageText = string.format("%d %s%s ago", c, UnitName, c > 1 and "s" or "")
			break
		end
	end

	return ageText
end

local ChangesDrawer = Roact.Component:extend("ConnectedPage")

function ChangesDrawer:init()
	-- Hold onto the serve session during the lifecycle of this component
	-- so that it can still render during the fade out after disconnecting
	self.serveSession = self.props.serveSession
end

function ChangesDrawer:render()
	if self.props.rendered == false or self.serveSession == nil then
		return nil
	end

	return Theme.with(function(theme)
		return e(BorderedContainer, {
			transparency = self.props.transparency,
			size = self.props.height:map(function(y)
				return UDim2.new(1, 0, y, -180 * y)
			end),
			position = UDim2.new(0, 0, 1, 0),
			anchorPoint = Vector2.new(0, 1),
			layoutOrder = self.props.layoutOrder,
		}, {
			Close = e(IconButton, {
				icon = Assets.Images.Icons.Close,
				iconSize = 24,
				color = theme.ConnectionDetails.DisconnectColor,
				transparency = self.props.transparency,

				position = UDim2.new(1, 0, 0, 0),
				anchorPoint = Vector2.new(1, 0),

				onClick = self.props.onClose,
			}, {
				Tip = e(Tooltip.Trigger, {
					text = "Close the patch visualizer",
				}),
			}),

			PatchVisualizer = e(PatchVisualizer, {
				size = UDim2.new(1, 0, 1, 0),
				transparency = self.props.transparency,
				layoutOrder = 3,

				columnVisibility = { true, false, true },
				patch = self.props.patch,
				instanceMap = self.serveSession.__instanceMap,
			}),
		})
	end)
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
				BackgroundTransparency = 1,
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
					text = "Disconnect from the Rojo sync server",
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

function ConnectedPage:getChangeInfoText()
	local patchData = self.props.patchData
	if patchData == nil then
		return ""
	end

	local elapsed = os.time() - patchData.timestamp
	local unapplied = PatchSet.countChanges(patchData.unapplied)

	return
		"<i>Synced "
		.. timeSinceText(elapsed)
		.. (if unapplied > 0 then
			string.format(
				", <font color=\"#FF8E3C\">but %d change%s failed to apply</font>",
				unapplied,
				unapplied == 1 and "" or "s"
			)
		else "")
		.. "</i>"
end

function ConnectedPage:startChangeInfoTextUpdater()
	-- Cancel any existing updater
	self:stopChangeInfoTextUpdater()

	-- Start a new updater
	self.changeInfoTextUpdater = task.defer(function()
		while true do
			self.setChangeInfoText(self:getChangeInfoText())

			local elapsed = os.time() - self.props.patchData.timestamp
			local updateInterval = 1

			-- Update timestamp text as frequently as currently needed
			for _, UnitData in ipairs(AGE_UNITS) do
				local UnitSeconds = UnitData[1]
				if elapsed > UnitSeconds then
					updateInterval = UnitSeconds
					break
				end
			end

			task.wait(updateInterval)
		end
	end)
end

function ConnectedPage:stopChangeInfoTextUpdater()
	if self.changeInfoTextUpdater then
		task.cancel(self.changeInfoTextUpdater)
		self.changeInfoTextUpdater = nil
	end
end

function ConnectedPage:init()
	self.changeDrawerMotor = Flipper.SingleMotor.new(0)
	self.changeDrawerHeight = bindingUtil.fromMotor(self.changeDrawerMotor)

	self.changeDrawerMotor:onStep(function(value)
		local renderChanges = value > 0.05

		self:setState(function(state)
			if state.renderChanges == renderChanges then
				return nil
			end

			return {
				renderChanges = renderChanges,
			}
		end)
	end)

	self:setState({
		renderChanges = false,
	})

	self.changeInfoText, self.setChangeInfoText = Roact.createBinding("")

	self:startChangeInfoTextUpdater()
end

function ConnectedPage:willUnmount()
	self:stopChangeInfoTextUpdater()
end

function ConnectedPage:didUpdate(previousProps)
	if self.props.patchData.timestamp ~= previousProps.patchData.timestamp then
		self:startChangeInfoTextUpdater()
	end
end

function ConnectedPage:render()
	return Theme.with(function(theme)
		return Roact.createFragment({
			Padding = e("UIPadding", {
				PaddingLeft = UDim.new(0, 20),
				PaddingRight = UDim.new(0, 20),
			}),

			Layout = e("UIListLayout", {
				VerticalAlignment = Enum.VerticalAlignment.Center,
				FillDirection = Enum.FillDirection.Vertical,
				SortOrder = Enum.SortOrder.LayoutOrder,
				Padding = UDim.new(0, 10),
			}),

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

			ChangeInfo = e("TextButton", {
				Text = self.changeInfoText,
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

				[Roact.Event.Activated] = function()
					if self.state.renderChanges then
						self.changeDrawerMotor:setGoal(Flipper.Spring.new(0, {
							frequency = 4,
							dampingRatio = 1,
						}))
					else
						self.changeDrawerMotor:setGoal(Flipper.Spring.new(1, {
							frequency = 3,
							dampingRatio = 1,
						}))
					end
				end,
			}),

			ChangesDrawer = e(ChangesDrawer, {
				rendered = self.state.renderChanges,
				transparency = self.props.transparency,
				patch = self.props.patchData.patch,
				serveSession = self.props.serveSession,
				height = self.changeDrawerHeight,
				layoutOrder = 4,

				onClose = function()
					self.changeDrawerMotor:setGoal(Flipper.Spring.new(0, {
						frequency = 4,
						dampingRatio = 1,
					}))
				end,
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
