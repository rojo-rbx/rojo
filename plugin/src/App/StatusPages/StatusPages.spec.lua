return function()
	local Rojo = script:FindFirstAncestor("Rojo")
	local Plugin = Rojo.Plugin
	local Roact = require(Rojo.Packages.Roact)

	local Assets = require(Plugin.Assets)
	local PatchSet = require(Plugin.PatchSet)
	local PatchTree = require(Plugin.PatchTree)
	local Theme = require(Plugin.App.Theme)
	local Tooltip = require(Plugin.App.Components.Tooltip)

	local Connected = require(script.Parent.Connected)
	local Confirming = require(script.Parent.Confirming)
	local Connecting = require(script.Parent.Connecting)
	local Error = require(script.Parent.Error)
	local NotConnected = require(script.Parent.NotConnected)
	local SettingsPage = require(script.Parent.Settings)

	local e = Roact.createElement
	local transparency = select(1, Roact.createBinding(0))

	local function findText(root, text)
		for _, object in root:GetDescendants() do
			if
				(object:IsA("TextLabel") or object:IsA("TextButton") or object:IsA("TextBox"))
				and object.Text == text
			then
				return object
			end
		end
		return nil
	end

	local function expectNoVisibleRojo(root)
		for _, object in root:GetDescendants() do
			if object:IsA("TextLabel") or object:IsA("TextButton") or object:IsA("TextBox") then
				expect(string.find(string.lower(object.Text), "rojo", 1, true)).to.equal(nil)
			end
		end
	end

	local function mountPage(component, props, size)
		local target = Instance.new("Frame")
		target.Size = UDim2.fromOffset(size.X, size.Y)
		local handle = Roact.mount(
			e(Theme.StudioProvider, nil, {
				Tooltips = e(Tooltip.Provider, nil, {
					Container = e(Tooltip.Container, nil),
					Page = e(component, props),
				}),
			}),
			target
		)
		task.wait()
		return target, handle
	end

	local function unmountPage(target, handle)
		Roact.unmount(handle)
		target:Destroy()
	end

	describe("Prism status pages", function()
		it("keeps the disconnected workflow functional in a narrow panel", function()
			local hostChanged = function() end
			local portChanged = function() end
			local target, handle = mountPage(NotConnected, {
				host = "127.0.0.1",
				port = "34872",
				onHostChange = hostChanged,
				onPortChange = portChanged,
				onConnect = function() end,
				onNavigateSettings = function() end,
				transparency = transparency,
			}, Vector2.new(300, 210))

			local wordmark = target:FindFirstChild("Wordmark", true)
			expect(wordmark).to.be.ok()
			expect(wordmark.Image).to.equal(Assets.Images.FullLogo)
			expect(findText(target, "Studio automation for Roblox")).to.be.ok()
			expect(findText(target, "Disconnected")).to.be.ok()
			expect(findText(target, "127.0.0.1")).to.be.ok()
			expect(findText(target, "34872")).to.be.ok()
			expect(findText(target, "Connect")).to.be.ok()
			expect(findText(target, "Settings")).to.be.ok()
			expectNoVisibleRojo(target)

			expect(wordmark.Size.X.Scale).to.equal(1)
			expect(wordmark.Size.X.Offset).to.equal(-56)
			unmountPage(target, handle)
		end)

		it("passes the existing connect and settings actions through unchanged", function()
			local connectCalls = 0
			local settingsCalls = 0
			local consumer = NotConnected.render({
				props = {
					onConnect = function()
						connectCalls += 1
					end,
					onNavigateSettings = function()
						settingsCalls += 1
					end,
					transparency = transparency,
				},
			})
			local root = consumer.props.render({
				Font = {
					Thin = Font.fromEnum(Enum.Font.SourceSans),
				},
				TextSize = {
					Small = 13,
				},
				Tokens = Theme.Tokens,
			})
			local children = root.props[Roact.Children]
			local buttons = children.Buttons.props[Roact.Children]

			buttons.Connect.props.onClick()
			buttons.Settings.props.onClick()
			expect(connectCalls).to.equal(1)
			expect(settingsCalls).to.equal(1)
		end)

		it("shows a compact connected summary without empty dead space", function()
			local target, handle = mountPage(Connected, {
				projectName = "PrismProject",
				address = "localhost:34872",
				patchData = {
					patch = PatchSet.newEmpty(),
					unapplied = PatchSet.newEmpty(),
					timestamp = DateTime.now().UnixTimestamp,
				},
				patchTree = nil,
				serveSession = nil,
				onDisconnect = function() end,
				onNavigateSettings = function() end,
				transparency = transparency,
			}, Vector2.new(300, 210))

			expect(findText(target, "Prism")).to.be.ok()
			expect(findText(target, "Connected")).to.be.ok()
			expect(findText(target, "PrismProject")).to.be.ok()
			expect(findText(target, "localhost:34872")).to.be.ok()
			expect(findText(target, "Disconnect")).to.be.ok()
			expect(findText(target, "Settings")).to.be.ok()
			expectNoVisibleRojo(target)
			unmountPage(target, handle)
		end)

		it("brands the connecting and confirming layouts", function()
			local connectingTarget, connectingHandle = mountPage(Connecting, {
				text = "Waiting for server",
				transparency = transparency,
			}, Vector2.new(300, 210))
			expect(findText(connectingTarget, "Connecting to Prism server...")).to.be.ok()
			expect(connectingTarget:FindFirstChild("Logo", true).Image).to.equal(Assets.Images.Logo)
			expectNoVisibleRojo(connectingTarget)
			unmountPage(connectingTarget, connectingHandle)

			local patchTree = PatchTree.build(PatchSet.newEmpty(), {
				fromIds = {},
				fromInstances = {},
			}, { "Property", "Current", "Incoming" })
			local confirmingTarget, confirmingHandle = mountPage(Confirming, {
				confirmData = {
					serverInfo = {
						projectName = "PrismProject",
					},
				},
				patchTree = patchTree,
				createPopup = false,
				onAbort = function() end,
				onAccept = function() end,
				onReject = function() end,
				transparency = transparency,
			}, Vector2.new(500, 350))
			expect(findText(confirmingTarget, "Review Prism sync changes for 'PrismProject'")).to.be.ok()
			expect(confirmingTarget:FindFirstChild("Logo", true).Image).to.equal(Assets.Images.PluginButtonWarning)
			expect(findText(confirmingTarget, "Accept")).to.be.ok()
			expect(findText(confirmingTarget, "Abort")).to.be.ok()
			unmountPage(confirmingTarget, confirmingHandle)
		end)

		it("uses the Prism warning asset for visible errors", function()
			local target, handle = mountPage(Error, {
				errorMessage = "Connection refused",
				onClose = function() end,
				transparency = transparency,
			}, Vector2.new(300, 210))

			expect(findText(target, "Connection refused")).to.be.ok()
			expect(target:FindFirstChild("Logo", true).Image).to.equal(Assets.Images.PluginButtonWarning)
			expect(findText(target, "Okay")).to.be.ok()
			expectNoVisibleRojo(target)
			unmountPage(target, handle)
		end)

		it("opens the existing settings surface with Prism branding", function()
			local target, handle = mountPage(SettingsPage, {
				syncActive = false,
				onBack = function() end,
				transparency = transparency,
			}, Vector2.new(300, 300))

			expect(findText(target, "Prism Settings")).to.be.ok()
			expect(target:FindFirstChild("Logo", true).Image).to.equal(Assets.Images.Logo)
			unmountPage(target, handle)
		end)
	end)
end
