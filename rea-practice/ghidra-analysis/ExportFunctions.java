import ghidra.app.script.GhidraScript;
import ghidra.program.model.listing.*;
import ghidra.program.model.symbol.*;
import java.io.*;
import java.util.*;

public class ExportFunctions extends GhidraScript {
    @Override
    public void run() throws Exception {
        String outputDir = "/home/z/my-project/rea-practice/ghidra-analysis/";
        
        // Export function list
        PrintWriter funcWriter = new PrintWriter(outputDir + "functions.txt");
        FunctionManager fm = currentProgram.getFunctionManager();
        FunctionIterator funcs = fm.getFunctions(true);
        int count = 0;
        while (funcs.hasNext()) {
            Function f = funcs.next();
            funcWriter.println(f.getName() + " @ " + f.getEntryPoint() + " size=" + f.getBody().getNumAddresses());
            count++;
        }
        funcWriter.close();
        println("Exported " + count + " functions to functions.txt");
        
        // Export symbol table
        PrintWriter symWriter = new PrintWriter(outputDir + "symbols.txt");
        SymbolTable st = currentProgram.getSymbolTable();
        SymbolIterator syms = st.getAllSymbols(true);
        int symCount = 0;
        while (syms.hasNext()) {
            Symbol s = syms.next();
            symWriter.println(s.getSymbolType() + " | " + s.getName() + " @ " + s.getAddress());
            symCount++;
            if (symCount > 50000) break; // Cap at 50k
        }
        symWriter.close();
        println("Exported " + symCount + " symbols to symbols.txt");
        
        // Export strings
        PrintWriter strWriter = new PrintWriter(outputDir + "strings.txt");
        DataIterator data = currentProgram.getListing().getDefinedData(true);
        int strCount = 0;
        while (data.hasNext()) {
            ghidra.program.model.data.Data d = data.next();
            if (d.hasStringValue()) {
                String val = d.getValue().toString();
                if (val.length() > 5) {
                    strWriter.println(d.getAddress() + " | " + val.substring(0, Math.min(val.length(), 200)));
                    strCount++;
                }
            }
            if (strCount > 10000) break;
        }
        strWriter.close();
        println("Exported " + strCount + " strings to strings.txt");
        
        // Export call graph (caller → callee relationships)
        PrintWriter callWriter = new PrintWriter(outputDir + "callgraph.txt");
        FunctionIterator funcIter = fm.getFunctions(true);
        int edgeCount = 0;
        while (funcIter.hasNext()) {
            Function f = funcIter.next();
            for (Function called : f.getCalledFunctions(null)) {
                callWriter.println(f.getName() + " -> " + called.getName());
                edgeCount++;
                if (edgeCount > 20000) break;
            }
        }
        callWriter.close();
        println("Exported " + edgeCount + " call edges to callgraph.txt");
    }
}
