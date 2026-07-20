-- Temporary development-only entry point for manually verifying ExecSpike in
-- a real installed plugin context. This is intentionally isolated from App and
-- ServeSession and should be removed with the feasibility spike.
if not plugin then
	return
end

local ExecSpike = require(script.Parent.ExecSpike)

local manualCases = {
	{ name = "readOnly", label = "Inspect" },
	{ name = "createPart", label = "Part" },
	{ name = "attachmentAndAttribute", label = "Attachment" },
	{ name = "compileFailure", label = "Compile Fail" },
	{ name = "runtimeFailure", label = "Runtime Fail" },
	{ name = "yielding", label = "Yield" },
}

local toolbar = plugin:CreateToolbar("Rojo Exec Spike")
local buttons = {}

local function findRemainingTemporaryModules()
	local remaining = {}
	local function collect(container)
		for _, descendant in container:GetDescendants() do
			if descendant:IsA("ModuleScript") and descendant.Name:find("^__RojoExecSpike_") then
				table.insert(remaining, descendant:GetFullName())
			end
		end
	end

	collect(game)
	collect(script.Parent.ExecSpike)

	return remaining
end

local function report(caseName, result)
	print(
		string.format(
			"[Rojo Exec Spike] %s: ok=%s phase=%s result=%s",
			caseName,
			tostring(result.ok),
			tostring(result.phase),
			tostring(result.result)
		)
	)

	if result.error ~= nil then
		warn("[Rojo Exec Spike] error: " .. result.error)
	end
	if result.traceback ~= nil then
		warn("[Rojo Exec Spike] traceback:\n" .. result.traceback)
	end
	if result.cleanupError ~= nil then
		warn("[Rojo Exec Spike] cleanup error: " .. result.cleanupError)
	end

	local remaining = findRemainingTemporaryModules()
	if #remaining == 0 then
		print("[Rojo Exec Spike] cleanup check: no temporary ModuleScripts remain")
	else
		warn("[Rojo Exec Spike] cleanup check failed: " .. table.concat(remaining, ", "))
	end
end

for _, manualCase in manualCases do
	local caseName = manualCase.name
	local button = toolbar:CreateButton(
		"RojoExecSpike_" .. caseName,
		"Run the " .. manualCase.label .. " rojo exec feasibility case",
		"",
		manualCase.label
	)
	button.ClickableWhenViewportHidden = true
	button.Click:Connect(function()
		local result = ExecSpike.run(ExecSpike.ManualSources[caseName], caseName .. ".lua")
		report(caseName, result)
	end)
	table.insert(buttons, button)
end

plugin.Unloading:Connect(function()
	for _, button in buttons do
		button:Destroy()
	end
	toolbar:Destroy()
end)
