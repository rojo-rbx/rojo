return {
	WrapInFunction       = require("prometheus.steps.WrapInFunction");
	SplitStrings         = require("prometheus.steps.SplitStrings");
	Vmify                = require("prometheus.steps.Vmify");
	ConstantArray        = require("prometheus.steps.ConstantArray");
	ProxifyLocals  			 = require("prometheus.steps.ProxifyLocals");
	AntiTamper  				 = require("prometheus.steps.AntiTamper");
	EncryptStrings 			 = require("prometheus.steps.EncryptStrings");
	NumbersToExpressions = require("prometheus.steps.NumbersToExpressions");
	AddVararg 					 = require("prometheus.steps.AddVararg");
	WatermarkCheck		   = require("prometheus.steps.WatermarkCheck");
}