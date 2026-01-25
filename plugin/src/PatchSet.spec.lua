return function()
	local PatchSet = require(script.Parent.PatchSet)
	local InstanceMap = require(script.Parent.InstanceMap)

	describe("newEmpty", function()
		it("should create an empty patch", function()
			local patch = PatchSet.newEmpty()
			expect(PatchSet.isEmpty(patch)).to.equal(true)
		end)
	end)

	describe("isEmpty", function()
		it("should return true for empty patches", function()
			local patch = PatchSet.newEmpty()
			expect(PatchSet.isEmpty(patch)).to.equal(true)
		end)

		it("should return false when patch has removals", function()
			local patch = PatchSet.newEmpty()
			table.insert(patch.removed, "some-id")
			expect(PatchSet.isEmpty(patch)).to.equal(false)
		end)

		it("should return false when patch has additions", function()
			local patch = PatchSet.newEmpty()
			patch.added["some-id"] = { Id = "some-id", ClassName = "Folder", Name = "Test" }
			expect(PatchSet.isEmpty(patch)).to.equal(false)
		end)

		it("should return false when patch has updates", function()
			local patch = PatchSet.newEmpty()
			table.insert(patch.updated, { id = "some-id", changedProperties = {} })
			expect(PatchSet.isEmpty(patch)).to.equal(false)
		end)
	end)

	describe("merge", function()
		it("should merge additions from source into target", function()
			local target = PatchSet.newEmpty()
			local source = PatchSet.newEmpty()

			source.added["CHILD"] = {
				Id = "CHILD",
				ClassName = "Folder",
				Name = "Child",
				Parent = "ROOT",
				Children = {},
				Properties = {},
			}

			PatchSet.merge(target, source)

			expect(target.added["CHILD"]).to.be.ok()
			expect(target.added["CHILD"].Name).to.equal("Child")
		end)

		it("should merge removals from source into target", function()
			local target = PatchSet.newEmpty()
			local source = PatchSet.newEmpty()

			table.insert(source.removed, "CHILD")

			PatchSet.merge(target, source)

			expect(#target.removed).to.equal(1)
			expect(target.removed[1]).to.equal("CHILD")
		end)

		it("should cancel additions when source removes them", function()
			local target = PatchSet.newEmpty()
			target.added["CHILD"] = {
				Id = "CHILD",
				ClassName = "Folder",
				Name = "Child",
				Parent = "ROOT",
				Children = {},
				Properties = {},
			}

			local source = PatchSet.newEmpty()
			table.insert(source.removed, "CHILD")

			PatchSet.merge(target, source)

			-- The addition should be cancelled
			expect(target.added["CHILD"]).to.equal(nil)
			-- And no removal should be added since it was never actually applied
			expect(#target.removed).to.equal(0)
		end)

		it("should merge updates from source into target", function()
			local target = PatchSet.newEmpty()
			local source = PatchSet.newEmpty()

			table.insert(source.updated, {
				id = "INSTANCE",
				changedName = "NewName",
				changedProperties = {},
			})

			PatchSet.merge(target, source)

			expect(#target.updated).to.equal(1)
			expect(target.updated[1].changedName).to.equal("NewName")
		end)

		it("should combine updates for the same instance", function()
			local target = PatchSet.newEmpty()
			table.insert(target.updated, {
				id = "INSTANCE",
				changedName = "FirstName",
				changedProperties = {
					Value = { String = "First" },
				},
			})

			local source = PatchSet.newEmpty()
			table.insert(source.updated, {
				id = "INSTANCE",
				changedName = "SecondName",
				changedProperties = {
					OtherValue = { String = "Second" },
				},
			})

			PatchSet.merge(target, source)

			-- Should still only have one update entry
			expect(#target.updated).to.equal(1)
			-- Name should be overwritten by the newer update
			expect(target.updated[1].changedName).to.equal("SecondName")
			-- Both properties should be present
			expect(target.updated[1].changedProperties.Value).to.be.ok()
			expect(target.updated[1].changedProperties.OtherValue).to.be.ok()
		end)

		it("should remove update when reverted to current instance state", function()
			-- Create an instance to compare against
			local testInstance = Instance.new("StringValue")
			testInstance.Name = "OriginalName"
			testInstance.Value = "OriginalValue"

			local instanceMap = InstanceMap.new()
			instanceMap:insert("INSTANCE", testInstance)

			-- Target has an update changing the name
			local target = PatchSet.newEmpty()
			table.insert(target.updated, {
				id = "INSTANCE",
				changedName = "ChangedName",
				changedProperties = {},
			})

			-- Source reverts the name back to original
			local source = PatchSet.newEmpty()
			table.insert(source.updated, {
				id = "INSTANCE",
				changedName = "OriginalName",
				changedProperties = {},
			})

			PatchSet.merge(target, source, instanceMap)

			-- The update should be removed entirely since it matches current state
			expect(#target.updated).to.equal(0)

			testInstance:Destroy()
			instanceMap:stop()
		end)

		it("should remove individual property changes when reverted", function()
			-- Create an instance to compare against
			local testInstance = Instance.new("StringValue")
			testInstance.Name = "Test"
			testInstance.Value = "OriginalValue"

			local instanceMap = InstanceMap.new()
			instanceMap:insert("INSTANCE", testInstance)

			-- Target has property change
			local target = PatchSet.newEmpty()
			table.insert(target.updated, {
				id = "INSTANCE",
				changedName = "NewName",
				changedProperties = {
					Value = { String = "ChangedValue" },
				},
			})

			-- Source reverts the property back to original
			local source = PatchSet.newEmpty()
			table.insert(source.updated, {
				id = "INSTANCE",
				changedProperties = {
					Value = { String = "OriginalValue" },
				},
			})

			PatchSet.merge(target, source, instanceMap)

			-- Should still have update for the name change
			expect(#target.updated).to.equal(1)
			expect(target.updated[1].changedName).to.equal("NewName")
			-- But the Value property should be removed since it matches current
			expect(target.updated[1].changedProperties.Value).to.equal(nil)

			testInstance:Destroy()
			instanceMap:stop()
		end)

		it("should keep updates for different instances separate", function()
			local target = PatchSet.newEmpty()
			table.insert(target.updated, {
				id = "INSTANCE_A",
				changedName = "NameA",
				changedProperties = {},
			})

			local source = PatchSet.newEmpty()
			table.insert(source.updated, {
				id = "INSTANCE_B",
				changedName = "NameB",
				changedProperties = {},
			})

			PatchSet.merge(target, source)

			expect(#target.updated).to.equal(2)
		end)
	end)

	describe("assign", function()
		it("should merge multiple patches additively", function()
			local target = PatchSet.newEmpty()

			local source1 = PatchSet.newEmpty()
			source1.added["A"] = { Id = "A", ClassName = "Folder", Name = "A" }

			local source2 = PatchSet.newEmpty()
			source2.added["B"] = { Id = "B", ClassName = "Folder", Name = "B" }

			PatchSet.assign(target, source1, source2)

			expect(target.added["A"]).to.be.ok()
			expect(target.added["B"]).to.be.ok()
		end)
	end)

	describe("countChanges", function()
		it("should count property changes in additions", function()
			local patch = PatchSet.newEmpty()
			patch.added["A"] = {
				Id = "A",
				ClassName = "StringValue",
				Name = "A",
				Properties = {
					Value = { String = "test" },
					MaxValue = { Float64 = 100 },
				},
			}

			expect(PatchSet.countChanges(patch)).to.equal(2)
		end)

		it("should count removals as single changes", function()
			local patch = PatchSet.newEmpty()
			table.insert(patch.removed, "A")
			table.insert(patch.removed, "B")

			expect(PatchSet.countChanges(patch)).to.equal(2)
		end)

		it("should count property updates", function()
			local patch = PatchSet.newEmpty()
			table.insert(patch.updated, {
				id = "A",
				changedProperties = {
					Value = { String = "test" },
				},
			})

			expect(PatchSet.countChanges(patch)).to.equal(1)
		end)

		it("should count name changes", function()
			local patch = PatchSet.newEmpty()
			table.insert(patch.updated, {
				id = "A",
				changedName = "NewName",
				changedProperties = {},
			})

			expect(PatchSet.countChanges(patch)).to.equal(1)
		end)

		it("should count className changes", function()
			local patch = PatchSet.newEmpty()
			table.insert(patch.updated, {
				id = "A",
				changedClassName = "Model",
				changedProperties = {},
			})

			expect(PatchSet.countChanges(patch)).to.equal(1)
		end)
	end)

	describe("countInstances", function()
		it("should count all affected instances", function()
			local patch = PatchSet.newEmpty()
			patch.added["A"] = { Id = "A", ClassName = "Folder", Name = "A" }
			patch.added["B"] = { Id = "B", ClassName = "Folder", Name = "B" }
			table.insert(patch.removed, "C")
			table.insert(patch.updated, { id = "D", changedProperties = {} })

			expect(PatchSet.countInstances(patch)).to.equal(4)
		end)
	end)

	describe("isEqual", function()
		it("should return true for identical patches", function()
			local patchA = PatchSet.newEmpty()
			patchA.added["A"] = { Id = "A", ClassName = "Folder", Name = "A" }

			local patchB = PatchSet.newEmpty()
			patchB.added["A"] = { Id = "A", ClassName = "Folder", Name = "A" }

			expect(PatchSet.isEqual(patchA, patchB)).to.equal(true)
		end)

		it("should return false for different patches", function()
			local patchA = PatchSet.newEmpty()
			patchA.added["A"] = { Id = "A", ClassName = "Folder", Name = "A" }

			local patchB = PatchSet.newEmpty()
			patchB.added["B"] = { Id = "B", ClassName = "Folder", Name = "B" }

			expect(PatchSet.isEqual(patchA, patchB)).to.equal(false)
		end)
	end)
end
