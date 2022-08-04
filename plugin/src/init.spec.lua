return function()
	it("should load all submodules", function()
		local function loadRecursive(container)
			if container:IsA("ModuleScript") and not container.Name:find("%.spec$") then
				require(container)
			end

			for _, child in ipairs(container:GetChildren()) do
				loadRecursive(child)
			end
		end

		loadRecursive(script.Parent)
	end)
end
