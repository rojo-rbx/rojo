return function()
	local encodePatchUpdate = require(script.Parent.encodePatchUpdate)

	it("should return an update when there are property changes", function()
		local part = Instance.new("Part")
		local properties = {
			CFrame = true,
			Color = true,
		}
		local update = encodePatchUpdate(part, "PART", properties)

		expect(update.id).to.equal("PART")
		expect(update.changedProperties.CFrame).to.be.ok()
		expect(update.changedProperties.Color).to.be.ok()
	end)

	it("should return nil when there are no property changes", function()
		local part = Instance.new("Part")
		local properties = {
			NonExistentProperty = true,
		}
		local update = encodePatchUpdate(part, "PART", properties)

		expect(update).to.equal(nil)
	end)

	it("should set changedName in the update when the instance's Name changes", function()
		local part = Instance.new("Part")
		local properties = {
			Name = true,
		}

		part.Name = "We'reGettingToTheCoolPart"

		local update = encodePatchUpdate(part, "PART", properties)

		expect(update.changedName).to.equal("We'reGettingToTheCoolPart")
	end)

	it("should recreate instance in the update when the instance's ClassName changes", function()
		local part = Instance.new("Part")
		local properties = {
			ClassName = true
		}

		local update = encodePatchUpdate(part, "PART", properties)

		expect(update.changedProperties.ClassName).to.be.ok()
		expect(update.requiresRecreate).to.equal(true)
	end)

	it("should correctly encode property values", function()
		local part = Instance.new("Part")
		local properties = {
			Position = true,
			Color = true,
		}

		part.Position = Vector3.new(0, 100, 0)
		part.Color = Color3.new(0.8, 0.2, 0.9)

		local update = encodePatchUpdate(part, "PART", properties)
		local position = update.changedProperties.Position
		local color = update.changedProperties.Color

		expect(position.Vector3[1]).to.equal(0)
		expect(position.Vector3[2]).to.equal(100)
		expect(position.Vector3[3]).to.equal(0)

		expect(color.Color3[1]).to.be.near(0.8, 0.01)
		expect(color.Color3[2]).to.be.near(0.2, 0.01)
		expect(color.Color3[3]).to.be.near(0.9, 0.01)
	end)
end
