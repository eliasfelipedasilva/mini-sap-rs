// Ghidra Headless Script para descompilar rapidamente pontos críticos do Kernel SAP
// (main, nlsui_main, Dispatcher Dp*, TaskHandler Th*, Extended Memory Em*, DBSL Sybase, RFC)
// mesmo em binários gigantes como disp+work (87 MB) usando -noanalysis + DisassembleCommand.

import ghidra.app.cmd.disassemble.DisassembleCommand;
import ghidra.app.cmd.function.CreateFunctionCmd;
import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.FunctionManager;
import ghidra.program.model.symbol.Symbol;
import ghidra.program.model.symbol.SymbolIterator;
import ghidra.program.model.symbol.SymbolTable;
import ghidra.program.model.symbol.SymbolType;
import java.io.FileWriter;
import java.io.PrintWriter;
import java.util.ArrayList;
import java.util.List;

public class DecompileSapKernel extends GhidraScript {
    @Override
    public void run() throws Exception {
        String[] args = getScriptArgs();
        String outPath = (args.length > 0) ? args[0] : "sap_kernel_decompiled.txt";

        SymbolTable symTab = currentProgram.getSymbolTable();
        FunctionManager fm = currentProgram.getFunctionManager();
        DecompInterface decomp = new DecompInterface();
        decomp.openProgram(currentProgram);

        String[] targetPrefixes = new String[] {
            "main", "nlsui_main", "DpMain", "DpInit", "ThInit", "ThStartUp",
            "EmInit", "EmAlloc", "EsInit", "ab_init", "ab_rabax", "DbSl",
            "Rfc", "Ms", "Enq"
        };

        List<Symbol> matched = new ArrayList<>();
        int totalSymbols = 0;

        try (PrintWriter out = new PrintWriter(new FileWriter(outPath))) {
            out.println("=== GHIDRA SAP KERNEL DECOMPILATION REPORT ===");
            out.println("Binary       : " + currentProgram.getName());
            out.println("Architecture : " + currentProgram.getLanguageID());
            out.println("Image Base   : " + currentProgram.getImageBase());

            SymbolIterator symIter = symTab.getAllSymbols(true);
            while (symIter.hasNext() && !monitor.isCancelled()) {
                Symbol s = symIter.next();
                totalSymbols++;
                String name = s.getName();
                for (String pref : targetPrefixes) {
                    if (name.equals(pref) || name.startsWith(pref)) {
                        if (matched.size() < 250) {
                            matched.add(s);
                        }
                        break;
                    }
                }
            }

            out.println("Total Symbols: " + totalSymbols);
            out.println("Matched Key Symbols: " + matched.size());
            out.println("----------------------------------------------");

            for (Symbol s : matched) {
                out.printf("SYMBOL [%s] %s (%s)%n", s.getAddress(), s.getName(), s.getSymbolType());
            }

            out.println("\n=== DECOMPILED C FUNCTIONS (GHIDRA DECOMPILER) ===");
            int decompCount = 0;
            for (Symbol s : matched) {
                if (decompCount >= 18) break;
                String name = s.getName();
                Address addr = s.getAddress();
                if (addr == null || !addr.isMemoryAddress()) continue;

                Function fn = fm.getFunctionAt(addr);
                if (fn == null) {
                    DisassembleCommand disCmd = new DisassembleCommand(addr, null, true);
                    disCmd.applyTo(currentProgram, monitor);
                    CreateFunctionCmd fnCmd = new CreateFunctionCmd(addr);
                    fnCmd.applyTo(currentProgram, monitor);
                    fn = fm.getFunctionAt(addr);
                }

                if (fn != null && !fn.isExternal() && !fn.isThunk()) {
                    DecompileResults res = decomp.decompileFunction(fn, 20, monitor);
                    if (res != null && res.decompileCompleted() && res.getDecompiledFunction() != null) {
                        out.println("\n// ---------------------------------------------------------");
                        out.println("// Function: " + fn.getName() + " @ " + addr);
                        out.println("// ---------------------------------------------------------");
                        out.println(res.getDecompiledFunction().getC());
                        decompCount++;
                    }
                }
            }
        } finally {
            decomp.dispose();
        }
        println("Saved SAP Kernel Ghidra decompilation to: " + outPath);
    }
}
