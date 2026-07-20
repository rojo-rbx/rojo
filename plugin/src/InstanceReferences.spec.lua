return function()
	local InstanceReferences = require(script.Parent.InstanceReferences)

	describe("InstanceReferences", function()
		it("creates stable session-local references", function()
			local folder = Instance.new("Folder")
			folder.Parent = workspace
			local references = InstanceReferences.new("plugin-session")
			local first = references:reference(folder, "Workspace.Folder")
			local second = references:reference(folder, "Workspace.Renamed")
			expect(first.id).to.equal("pinst-00000001")
			expect(second.id).to.equal(first.id)
			expect(first.sessionId).to.equal("plugin-session")
			expect(references:resolve(first.id)).to.equal(folder)
			folder:Destroy()
			expect(references:resolve(first.id)).never.to.be.ok()
		end)

		it("clears the registry", function()
			local folder = Instance.new("Folder")
			folder.Parent = workspace
			local references = InstanceReferences.new("plugin-session")
			local reference = references:reference(folder, "Workspace.Folder")
			references:clear()
			expect(references:resolve(reference.id)).never.to.be.ok()
			folder:Destroy()
		end)
	end)
end
