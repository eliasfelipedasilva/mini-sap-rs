// Ghidra Headless Script (Java) para mapeamento acadêmico de símbolos, pontos de entrada
// e assinaturas de funções de binários ELF do Kernel SAP para interoperabilidade/estudo em Rust.
// Uso com analyzeHeadless:
//   analyzeHeadless <proj_dir> <proj_name> -import <binary> -postScript ExportSymbolsAndDecompile.java <out_json>

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.FunctionIterator;
import ghidra.program.model.listing.FunctionManager;
import java.io.FileWriter;
import java.io.PrintWriter;

public class ExportSymbolsAndDecompile extends GhidraScript {
    @Override
    public void run() throws Exception {
        String[] args = getScriptArgs();
        String outPath = (args.length > 0) ? args[0] : "ghidra_analysis.txt";

        FunctionManager fm = currentProgram.getFunctionManager();
        DecompInterface decomp = new DecompInterface();
        decomp.openProgram(currentProgram);

        try (PrintWriter out = new PrintWriter(new FileWriter(outPath))) {
            out.println("=== GHIDRA HEADLESS ARCHITECTURAL SUMMARY ===");
            out.println("Program Name : " + currentProgram.getName());
            out.println("Language ID  : " + currentProgram.getLanguageID());
            out.println("Compiler Spec: " + currentProgram.getCompilerSpec().getCompilerSpecID());
            out.println("Total Funcs  : " + fm.getFunctionCount());
            out.println("---------------------------------------------");

            FunctionIterator iter = fm.getFunctions(true);
            int count = 0;
            int decompiledCount = 0;
            while (iter.hasNext() && !monitor.isCancelled()) {
                Function fn = iter.next();
                if (fn.isExternal() || fn.isThunk()) {
                    continue;
                }
                count++;
                String name = fn.getName();
                if (count <= 100 || name.contains("main") || name.contains("ab_") || name.contains("Th") || name.contains("Dp") || name.contains("Rfc")) {
                    out.printf("FUNC [%s] %s (%d bytes) -> sig: %s%n",
                            fn.getEntryPoint().toString(),
                            name,
                            fn.getBody().getNumAddresses(),
                            fn.getSignature().getPrototypeString());
                }
                if (decompiledCount < 15 && (name.equals("main") || name.startsWith("nlsui_main") || name.startsWith("Dp") || name.startsWith("Th") || decompiledCount < 5)) {
                    DecompileResults res = decomp.decompileFunction(fn, 15, monitor);
                    if (res != null && res.decompileCompleted() && res.getDecompiledFunction() != null) {
                        out.println("--- DECOMPILED: " + name + " ---");
                        out.println(res.getDecompiledFunction().getC());
                        decompiledCount++;
                    }
                }
            }
        } finally {
            decomp.dispose();
        }
        println("Exported analysis to: " + outPath);
    }
}
