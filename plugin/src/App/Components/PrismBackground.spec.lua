return function()
	local Rojo = script:FindFirstAncestor("Rojo")
	local Roact = require(Rojo.Packages.Roact)

	local Theme = require(Rojo.Plugin.App.Theme)
	local PrismBackground = require(script.Parent.PrismBackground)

	local e = Roact.createElement

	local function createStepSignal()
		local state = {
			connections = 0,
			disconnections = 0,
			callback = nil,
		}
		local signal = {}

		function signal:Connect(callback)
			state.connections += 1
			state.callback = callback
			local connected = true
			return {
				Disconnect = function()
					if connected then
						connected = false
						state.disconnections += 1
					end
				end,
			}
		end

		return signal, state
	end

	local function createTree(stepSignal, props)
		return e(Theme.StudioProvider, nil, {
			Background = e(PrismBackground, {
				active = props.active,
				reducedMotion = props.reducedMotion,
				stepSignal = stepSignal,
			}),
		})
	end

	describe("PrismBackground", function()
		it("uses five independently timed ambient blobs", function()
			expect(#PrismBackground._test.blobs).to.equal(5)
			for _, blob in PrismBackground._test.blobs do
				expect(blob.period >= 15).to.equal(true)
				expect(blob.period <= 40).to.equal(true)
			end

			local startPosition = PrismBackground._test.getTransform(PrismBackground._test.blobs[1], 0)
			local laterPosition = PrismBackground._test.getTransform(PrismBackground._test.blobs[1], 4)
			expect(laterPosition == startPosition).to.equal(false)
		end)

		it("connects once while active and disconnects when hidden", function()
			local signal, state = createStepSignal()
			local target = Instance.new("ScreenGui")
			local handle = Roact.mount(
				createTree(signal, {
					active = true,
					reducedMotion = false,
				}),
				target
			)

			expect(state.connections).to.equal(1)
			expect(state.callback).to.be.ok()
			state.callback(1 / 60)

			Roact.update(
				handle,
				createTree(signal, {
					active = false,
					reducedMotion = false,
				})
			)
			expect(state.disconnections).to.equal(1)

			Roact.unmount(handle)
			expect(state.disconnections).to.equal(1)
			target:Destroy()
		end)

		it("uses a static fallback when reduced motion is requested", function()
			local signal, state = createStepSignal()
			local target = Instance.new("ScreenGui")
			local handle = Roact.mount(
				createTree(signal, {
					active = true,
					reducedMotion = true,
				}),
				target
			)

			expect(state.connections).to.equal(0)
			Roact.unmount(handle)
			expect(state.disconnections).to.equal(0)
			target:Destroy()
		end)
	end)
end
