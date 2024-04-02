local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local Theme = require(Plugin.App.Theme)
local Assets = require(Plugin.Assets)
local PatchSet = require(Plugin.PatchSet)

local StudioPluginGui = require(Plugin.App.Components.Studio.StudioPluginGui)
local Header = require(Plugin.App.Components.Header)
local IconButton = require(Plugin.App.Components.IconButton)
local TextButton = require(Plugin.App.Components.TextButton)
local BorderedContainer = require(Plugin.App.Components.BorderedContainer)
local Tooltip = require(Plugin.App.Components.Tooltip)
local PatchVisualizer = require(Plugin.App.Components.PatchVisualizer)
local StringDiffVisualizer = require(Plugin.App.Components.StringDiffVisualizer)
local TableDiffVisualizer = require(Plugin.App.Components.TableDiffVisualizer)

local e = Roact.createElement

local AGE_UNITS = {
	{ 31556909, "y" },
	{ 2629743, "mon" },
	{ 604800, "w" },
	{ 86400, "d" },
	{ 3600, "h" },
	{ 60, "m" },
}
function timeSinceText(elapsed: number): string
	local ageText = string.format("%ds", elapsed)

	for _, UnitData in ipairs(AGE_UNITS) do
		local UnitSeconds, UnitName = UnitData[1], UnitData[2]
		if elapsed > UnitSeconds then
			ageText = elapsed // UnitSeconds .. UnitName
			break
		end
	end

	return ageText
end

local ChangesViewer = Roact.Component:extend("ChangesViewer")

function ChangesViewer:init()
	-- Hold onto the serve session during the lifecycle of this component
	-- so that it can still render during the fade out after disconnecting
	self.serveSession = self.props.serveSession
end

function ChangesViewer:render()
	if self.props.rendered == false or self.serveSession == nil or self.props.patchData == nil then
		return nil
	end

	local unapplied = PatchSet.countChanges(self.props.patchData.unapplied)
	local applied = PatchSet.countChanges(self.props.patchData.patch) - unapplied

	return Theme.with(function(theme)
		return Roact.createFragment({
			Navbar = e("Frame", {
				Size = UDim2.new(1, 0, 0, 40),
				BackgroundTransparency = 1,
			}, {
				Close = e(IconButton, {
					icon = Assets.Images.Icons.Close,
					iconSize = 24,
					color = theme.Settings.Navbar.BackButtonColor,
					transparency = self.props.transparency,

					position = UDim2.new(0, 0, 0.5, 0),
					anchorPoint = Vector2.new(0, 0.5),

					onClick = self.props.onBack,
				}, {
					Tip = e(Tooltip.Trigger, {
						text = "Close",
					}),
				}),

				Title = e("TextLabel", {
					Text = "Sync",
					Font = Enum.Font.GothamMedium,
					TextSize = 17,
					TextXAlignment = Enum.TextXAlignment.Left,
					TextColor3 = theme.TextColor,
					TextTransparency = self.props.transparency,
					Size = UDim2.new(1, -40, 0, 20),
					Position = UDim2.new(0, 40, 0, 0),
					BackgroundTransparency = 1,
				}),

				Subtitle = e("TextLabel", {
					Text = DateTime.fromUnixTimestamp(self.props.patchData.timestamp):FormatLocalTime("LTS", "en-us"),
					TextXAlignment = Enum.TextXAlignment.Left,
					Font = Enum.Font.Gotham,
					TextSize = 15,
					TextColor3 = theme.SubTextColor,
					TextTruncate = Enum.TextTruncate.AtEnd,
					TextTransparency = self.props.transparency,
					Size = UDim2.new(1, -40, 0, 16),
					Position = UDim2.new(0, 40, 0, 20),
					BackgroundTransparency = 1,
				}),

				Info = e("Frame", {
					BackgroundTransparency = 1,
					Size = UDim2.new(0, 10, 0, 24),
					AutomaticSize = Enum.AutomaticSize.X,
					Position = UDim2.new(1, -5, 0.5, 0),
					AnchorPoint = Vector2.new(1, 0.5),
				}, {
					Tooltip = e(Tooltip.Trigger, {
						text = `{applied} changes applied`
							.. (if unapplied > 0 then `, {unapplied} changes failed` else ""),
					}),
					Content = e("Frame", {
						BackgroundTransparency = 1,
						Size = UDim2.new(0, 0, 1, 0),
						AutomaticSize = Enum.AutomaticSize.X,
					}, {
						Layout = e("UIListLayout", {
							FillDirection = Enum.FillDirection.Horizontal,
							HorizontalAlignment = Enum.HorizontalAlignment.Right,
							VerticalAlignment = Enum.VerticalAlignment.Center,
							SortOrder = Enum.SortOrder.LayoutOrder,
							Padding = UDim.new(0, 4),
						}),

						StatusIcon = e("ImageLabel", {
							BackgroundTransparency = 1,
							Image = if unapplied > 0
								then Assets.Images.Icons.SyncWarning
								else Assets.Images.Icons.SyncSuccess,
							ImageColor3 = if unapplied > 0 then theme.Diff.Warning else theme.TextColor,
							Size = UDim2.new(0, 24, 0, 24),
							LayoutOrder = 10,
						}),
						StatusSpacer = e("Frame", {
							BackgroundTransparency = 1,
							Size = UDim2.new(0, 6, 0, 4),
							LayoutOrder = 9,
						}),
						AppliedIcon = e("ImageLabel", {
							BackgroundTransparency = 1,
							Image = Assets.Images.Icons.Checkmark,
							ImageColor3 = theme.TextColor,
							Size = UDim2.new(0, 16, 0, 16),
							LayoutOrder = 1,
						}),
						AppliedText = e("TextLabel", {
							Text = applied,
							Font = Enum.Font.Gotham,
							TextSize = 15,
							TextColor3 = theme.TextColor,
							TextTransparency = self.props.transparency,
							Size = UDim2.new(0, 0, 1, 0),
							AutomaticSize = Enum.AutomaticSize.X,
							BackgroundTransparency = 1,
							LayoutOrder = 2,
						}),
						Warnings = if unapplied > 0
							then Roact.createFragment({
								WarningsSpacer = e("Frame", {
									BackgroundTransparency = 1,
									Size = UDim2.new(0, 4, 0, 4),
									LayoutOrder = 3,
								}),
								UnappliedIcon = e("ImageLabel", {
									BackgroundTransparency = 1,
									Image = Assets.Images.Icons.Exclamation,
									ImageColor3 = theme.Diff.Warning,
									Size = UDim2.new(0, 4, 0, 16),
									LayoutOrder = 4,
								}),
								UnappliedText = e("TextLabel", {
									Text = unapplied,
									Font = Enum.Font.Gotham,
									TextSize = 15,
									TextColor3 = theme.Diff.Warning,
									TextTransparency = self.props.transparency,
									Size = UDim2.new(0, 0, 1, 0),
									AutomaticSize = Enum.AutomaticSize.X,
									BackgroundTransparency = 1,
									LayoutOrder = 5,
								}),
							})
							else nil,
					}),
				}),

				Divider = e("Frame", {
					BackgroundColor3 = theme.Settings.DividerColor,
					BackgroundTransparency = self.props.transparency,
					Size = UDim2.new(1, 0, 0, 1),
					Position = UDim2.new(0, 0, 1, 0),
					BorderSizePixel = 0,
				}, {
					Gradient = e("UIGradient", {
						Transparency = NumberSequence.new({
							NumberSequenceKeypoint.new(0, 1),
							NumberSequenceKeypoint.new(0.1, 0),
							NumberSequenceKeypoint.new(0.9, 0),
							NumberSequenceKeypoint.new(1, 1),
						}),
					}),
				}),
			}),

			Patch = e(PatchVisualizer, {
				size = UDim2.new(1, -10, 1, -65),
				position = UDim2.new(0, 5, 1, -5),
				anchorPoint = Vector2.new(0, 1),
				transparency = self.props.transparency,
				layoutOrder = self.props.layoutOrder,

				patchTree = self.props.patchTree,

				showStringDiff = self.props.showStringDiff,
				showTableDiff = self.props.showTableDiff,
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
	return timeSinceText(DateTime.now().UnixTimestamp - patchData.timestamp)
end

function ConnectedPage:startChangeInfoTextUpdater()
	-- Cancel any existing updater
	self:stopChangeInfoTextUpdater()

	-- Start a new updater
	self.changeInfoTextUpdater = task.defer(function()
		while true do
			self.setChangeInfoText(self:getChangeInfoText())

			local elapsed = DateTime.now().UnixTimestamp - self.props.patchData.timestamp
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
	self:setState({
		renderChanges = false,
		hoveringChangeInfo = false,
		showingStringDiff = false,
		oldString = "",
		newString = "",
	})

	self.changeInfoText, self.setChangeInfoText = Roact.createBinding("")

	self:startChangeInfoTextUpdater()
end

function ConnectedPage:willUnmount()
	self:stopChangeInfoTextUpdater()
end

function ConnectedPage:didUpdate(previousProps)
	if self.props.patchData.timestamp ~= previousProps.patchData.timestamp then
		-- New patch recieved
		self:startChangeInfoTextUpdater()
		self:setState({
			showingStringDiff = false,
		})
	end
end

function ConnectedPage:render()
	local syncWarning = self.props.patchData
		and self.props.patchData.unapplied
		and PatchSet.countChanges(self.props.patchData.unapplied) > 0

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

			Heading = e("Frame", {
				BackgroundTransparency = 1,
				Size = UDim2.new(1, 0, 0, 32),
			}, {
				Header = e(Header, {
					transparency = self.props.transparency,
				}),

				ChangeInfo = e("TextButton", {
					Text = "",
					Size = UDim2.new(0, 0, 1, 0),
					AutomaticSize = Enum.AutomaticSize.X,
					BackgroundColor3 = theme.BorderedContainer.BorderedColor,
					BackgroundTransparency = if self.state.hoveringChangeInfo then 0.7 else 1,
					BorderSizePixel = 0,
					Position = UDim2.new(1, -5, 0.5, 0),
					AnchorPoint = Vector2.new(1, 0.5),
					[Roact.Event.MouseEnter] = function()
						self:setState({
							hoveringChangeInfo = true,
						})
					end,
					[Roact.Event.MouseLeave] = function()
						self:setState({
							hoveringChangeInfo = false,
						})
					end,
					[Roact.Event.Activated] = function()
						self:setState(function(prevState)
							prevState = prevState or {}
							return {
								renderChanges = not prevState.renderChanges,
							}
						end)
					end,
				}, {
					Corner = e("UICorner", {
						CornerRadius = UDim.new(0, 5),
					}),
					Tooltip = e(Tooltip.Trigger, {
						text = if self.state.renderChanges then "Hide changes" else "View changes",
					}),
					Content = e("Frame", {
						BackgroundTransparency = 1,
						Size = UDim2.new(0, 0, 1, 0),
						AutomaticSize = Enum.AutomaticSize.X,
					}, {
						Layout = e("UIListLayout", {
							FillDirection = Enum.FillDirection.Horizontal,
							HorizontalAlignment = Enum.HorizontalAlignment.Center,
							VerticalAlignment = Enum.VerticalAlignment.Center,
							SortOrder = Enum.SortOrder.LayoutOrder,
							Padding = UDim.new(0, 5),
						}),
						Padding = e("UIPadding", {
							PaddingLeft = UDim.new(0, 5),
							PaddingRight = UDim.new(0, 5),
						}),
						Text = e("TextLabel", {
							BackgroundTransparency = 1,
							Text = self.changeInfoText,
							Font = Enum.Font.Gotham,
							TextSize = 15,
							TextColor3 = if syncWarning then theme.Diff.Warning else theme.Header.VersionColor,
							TextTransparency = self.props.transparency,
							TextXAlignment = Enum.TextXAlignment.Right,
							Size = UDim2.new(0, 0, 1, 0),
							AutomaticSize = Enum.AutomaticSize.X,
							LayoutOrder = 1,
						}),
						Icon = e("ImageLabel", {
							BackgroundTransparency = 1,
							Image = if syncWarning
								then Assets.Images.Icons.SyncWarning
								else Assets.Images.Icons.SyncSuccess,
							ImageColor3 = if syncWarning then theme.Diff.Warning else theme.Header.VersionColor,
							ImageTransparency = self.props.transparency,
							Size = UDim2.new(0, 24, 0, 24),
							LayoutOrder = 2,
						}),
					}),
				}),
			}),

			ConnectionDetails = e(ConnectionDetails, {
				projectName = self.state.projectName,
				address = self.state.address,
				transparency = self.props.transparency,
				layoutOrder = 2,

				onDisconnect = self.props.onDisconnect,
			}),

			Buttons = e("Frame", {
				Size = UDim2.new(1, 0, 0, 34),
				LayoutOrder = 3,
				BackgroundTransparency = 1,
				ZIndex = 2,
			}, {
				Settings = e(TextButton, {
					text = "Settings",
					style = "Bordered",
					transparency = self.props.transparency,
					layoutOrder = 1,
					onClick = self.props.onNavigateSettings,
				}, {
					Tip = e(Tooltip.Trigger, {
						text = "View and modify plugin settings",
					}),
				}),

				Disconnect = e(TextButton, {
					text = "Disconnect",
					style = "Solid",
					transparency = self.props.transparency,
					layoutOrder = 2,
					onClick = self.props.onDisconnect,
				}, {
					Tip = e(Tooltip.Trigger, {
						text = "Disconnect from the Rojo sync server",
					}),
				}),

				Layout = e("UIListLayout", {
					HorizontalAlignment = Enum.HorizontalAlignment.Right,
					FillDirection = Enum.FillDirection.Horizontal,
					SortOrder = Enum.SortOrder.LayoutOrder,
					Padding = UDim.new(0, 10),
				}),
			}),

			ChangesViewer = e(StudioPluginGui, {
				id = "Rojo_ChangesViewer",
				title = "View changes",
				active = self.state.renderChanges,
				isEphemeral = true,

				initDockState = Enum.InitialDockState.Float,
				overridePreviousState = true,
				floatingSize = Vector2.new(400, 500),
				minimumSize = Vector2.new(300, 300),

				zIndexBehavior = Enum.ZIndexBehavior.Sibling,

				onClose = function()
					self:setState({
						renderChanges = false,
					})
				end,
			}, {
				TooltipsProvider = e(Tooltip.Provider, nil, {
					Tooltips = e(Tooltip.Container, nil),
					Content = e("Frame", {
						Size = UDim2.fromScale(1, 1),
						BackgroundTransparency = 1,
					}, {
						Changes = e(ChangesViewer, {
							transparency = self.props.transparency,
							rendered = self.state.renderChanges,
							patchData = self.props.patchData,
							patchTree = self.props.patchTree,
							serveSession = self.props.serveSession,
							showStringDiff = function(oldString: string, newString: string)
								self:setState({
									showingStringDiff = true,
									oldString = oldString,
									newString = newString,
								})
							end,
							showTableDiff = function(oldTable: { [any]: any? }, newTable: { [any]: any? })
								self:setState({
									showingTableDiff = true,
									oldTable = oldTable,
									newTable = newTable,
								})
							end,
							onBack = function()
								self:setState({
									renderChanges = false,
								})
							end,
						}),
					}),
				}),
			}),

			StringDiff = e(StudioPluginGui, {
				id = "Rojo_ConnectedStringDiff",
				title = "String diff",
				active = self.state.showingStringDiff,
				isEphemeral = true,

				initDockState = Enum.InitialDockState.Float,
				overridePreviousState = false,
				floatingSize = Vector2.new(500, 350),
				minimumSize = Vector2.new(400, 250),

				zIndexBehavior = Enum.ZIndexBehavior.Sibling,

				onClose = function()
					self:setState({
						showingStringDiff = false,
					})
				end,
			}, {
				TooltipsProvider = e(Tooltip.Provider, nil, {
					Tooltips = e(Tooltip.Container, nil),
					Content = e("Frame", {
						Size = UDim2.fromScale(1, 1),
						BackgroundTransparency = 1,
					}, {
						e(StringDiffVisualizer, {
							size = UDim2.new(1, -10, 1, -10),
							position = UDim2.new(0, 5, 0, 5),
							anchorPoint = Vector2.new(0, 0),
							transparency = self.props.transparency,

							oldString = self.state.oldString,
							newString = self.state.newString,
						}),
					}),
				}),
			}),

			TableDiff = e(StudioPluginGui, {
				id = "Rojo_ConnectedTableDiff",
				title = "Table diff",
				active = self.state.showingTableDiff,
				isEphemeral = true,

				initDockState = Enum.InitialDockState.Float,
				overridePreviousState = false,
				floatingSize = Vector2.new(500, 350),
				minimumSize = Vector2.new(400, 250),

				zIndexBehavior = Enum.ZIndexBehavior.Sibling,

				onClose = function()
					self:setState({
						showingTableDiff = false,
					})
				end,
			}, {
				TooltipsProvider = e(Tooltip.Provider, nil, {
					Tooltips = e(Tooltip.Container, nil),
					Content = e("Frame", {
						Size = UDim2.fromScale(1, 1),
						BackgroundTransparency = 1,
					}, {
						e(TableDiffVisualizer, {
							size = UDim2.new(1, -10, 1, -10),
							position = UDim2.new(0, 5, 0, 5),
							anchorPoint = Vector2.new(0, 0),
							transparency = self.props.transparency,

							oldTable = self.state.oldTable,
							newTable = self.state.newTable,
						}),
					}),
				}),
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
