return function()
	it("should load all submodules", function()
		local function loadRecursive(container)
			if container:IsA("ModuleScript") and not container.Name:find("%.spec$") then
				local success, err = pcall(require, container)
				if not success then
					error(string.format("Failed to load '%s': %s", container.Name, err))
				end
			end

			for _, child in ipairs(container:GetChildren()) do
				loadRecursive(child)
			end
		end

		loadRecursive(script.Parent)
	end)
end
