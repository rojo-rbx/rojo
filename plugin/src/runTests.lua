return function(TestEZ)
	local Rojo = script.Parent.Parent

	TestEZ.TestBootstrap:run({ Rojo.Plugin, Rojo.Http, Rojo.Log, Rojo.RbxDom })
end