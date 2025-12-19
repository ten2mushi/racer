#!/usr/bin/env python3
"""
BLS Consensus Chain Analysis Tool

Analyzes delivered.jsonl logs from all nodes to:
1. Verify that all nodes received the same messages (total consensus)
2. Check if delivery order is consistent (chain preservation)
3. Identify any missing or duplicate messages
4. Provide statistics on consensus quality

python3 analyze_consensus.py logs
"""

import json
import os
import sys
from pathlib import Path
from collections import defaultdict
from typing import Dict, List, Set, Tuple

def load_node_deliveries(log_dir: Path) -> Dict[str, List[dict]]:
    """Load delivered messages from all node log files."""
    node_data = {}
    
    for node_dir in sorted(log_dir.iterdir()):
        if not node_dir.is_dir():
            continue
            
        delivered_file = node_dir / "delivered.jsonl"
        if not delivered_file.exists():
            print(f"⚠️  Warning: No delivered.jsonl for {node_dir.name}")
            continue
        
        messages = []
        with open(delivered_file, 'r') as f:
            for line in f:
                line = line.strip()
                if line:  # Skip empty lines
                    try:
                        messages.append(json.loads(line))
                    except json.JSONDecodeError as e:
                        print(f"⚠️  Warning: Failed to parse line in {node_dir.name}: {e}")
        
        node_data[node_dir.name] = messages
    
    return node_data

def analyze_consensus(node_data: Dict[str, List[dict]]) -> None:
    """Analyze consensus across all nodes."""
    
    if not node_data:
        print("❌ No node data found!")
        return
    
    print(f"\n{'='*80}")
    print(f"BLS CONSENSUS ANALYSIS ({len(node_data)} nodes)")
    print(f"{'='*80}\n")
    
    # 1. Message count per node
    print("📊 Message Delivery Counts:")
    print(f"{'─'*80}")
    counts = {node: len(msgs) for node, msgs in node_data.items()}
    for node in sorted(counts.keys()):
        print(f"  {node:15} : {counts[node]:4} messages")
    
    min_count = min(counts.values())
    max_count = max(counts.values())
    avg_count = sum(counts.values()) / len(counts)
    
    print(f"{'─'*80}")
    print(f"  Min: {min_count} | Max: {max_count} | Avg: {avg_count:.1f}")
    print()
    
    # 2. Check for total consensus (all nodes have same batch_ids)
    print("🔍 Checking Batch ID Consensus:")
    print(f"{'─'*80}")
    
    node_batch_sets = {}
    for node, msgs in node_data.items():
        batch_ids = {msg['batch_id'] for msg in msgs}
        node_batch_sets[node] = batch_ids
    
    # Find the union and intersection of all batch_ids
    all_batches = set().union(*node_batch_sets.values()) if node_batch_sets else set()
    common_batches = set.intersection(*node_batch_sets.values()) if node_batch_sets else set()
    
    consensus_rate = (len(common_batches) / len(all_batches) * 100) if all_batches else 0
    
    print(f"  Total unique batches: {len(all_batches)}")
    print(f"  Batches on all nodes: {len(common_batches)}")
    print(f"  Consensus rate:       {consensus_rate:.1f}%")
    
    # Find missing batches per node
    missing_by_node = {}
    for node, batch_set in node_batch_sets.items():
        missing = all_batches - batch_set
        if missing:
            missing_by_node[node] = missing
    
    if missing_by_node:
        print(f"\n  ⚠️  Nodes missing batches:")
        for node, missing in sorted(missing_by_node.items()):
            print(f"     {node}: missing {len(missing)} batches")
    else:
        print(f"\n  ✓ All nodes have the same set of batches!")
    
    print()
    
    # 3. Check delivery order consistency (chain preservation)
    print("⛓️  Checking Chain Order Consistency:")
    print(f"{'─'*80}")
    
    # Get the canonical order from the first node (or create one from sequence)
    reference_node = sorted(node_data.keys())[0]
    reference_order = [msg['batch_id'] for msg in node_data[reference_node]]
    
    # Compare order across all nodes
    order_matches = 0
    order_mismatches = {}
    
    for node, msgs in node_data.items():
        node_order = [msg['batch_id'] for msg in msgs]
        
        # Only compare up to the minimum length
        min_len = min(len(reference_order), len(node_order))
        mismatches = []
        
        for i in range(min_len):
            if reference_order[i] != node_order[i]:
                mismatches.append((i, reference_order[i], node_order[i]))
        
        if not mismatches:
            order_matches += 1
        else:
            order_mismatches[node] = mismatches
    
    print(f"  Reference node: {reference_node}")
    print(f"  Nodes with matching order: {order_matches}/{len(node_data)}")
    
    if order_mismatches:
        print(f"\n  ⚠️  Order mismatches detected:")
        for node, mismatches in sorted(order_mismatches.items()):
            print(f"     {node}: {len(mismatches)} position(s) differ from reference")
            if len(mismatches) <= 5:  # Show first 5 mismatches
                for pos, ref_batch, node_batch in mismatches:
                    print(f"       @ seq {pos}: expected {ref_batch}, got {node_batch}")
    else:
        print(f"  ✓ All nodes have identical delivery order!")
    
    print()
    
    # 4. Check for duplicates within each node
    print("🔎 Checking for Duplicate Deliveries:")
    print(f"{'─'*80}")
    
    duplicates_found = False
    for node, msgs in node_data.items():
        batch_ids = [msg['batch_id'] for msg in msgs]
        if len(batch_ids) != len(set(batch_ids)):
            duplicates_found = True
            duplicate_count = len(batch_ids) - len(set(batch_ids))
            print(f"  ⚠️  {node}: {duplicate_count} duplicate(s)")
    
    if not duplicates_found:
        print(f"  ✓ No duplicates found on any node!")
    
    print()
    
    # 5. Verify sequence numbers
    print("🔢 Checking Sequence Number Integrity:")
    print(f"{'─'*80}")
    
    seq_issues = False
    for node, msgs in node_data.items():
        sequences = [msg['seq'] for msg in msgs]
        expected_seq = list(range(1, len(msgs) + 1))
        
        if sequences != expected_seq:
            seq_issues = True
            print(f"  ⚠️  {node}: sequence numbers are not monotonic")
            # Find gaps
            missing = set(expected_seq) - set(sequences)
            if missing:
                print(f"      Missing: {sorted(list(missing))[:10]}")  # Show first 10
    
    if not seq_issues:
        print(f"  ✓ All nodes have monotonic sequence numbers!")
    
    print()
    
    # 6. Final verdict
    print(f"{'='*80}")
    print("📋 FINAL VERDICT:")
    print(f"{'='*80}")
    
    if consensus_rate == 100 and not order_mismatches and not duplicates_found and not seq_issues:
        print("✅ PERFECT CONSENSUS ACHIEVED!")
        print("   - All nodes delivered the same messages")
        print("   - Delivery order is identical across all nodes")
        print("   - No duplicates or sequence issues")
        print("   - BLS signature aggregation working correctly!")
    elif consensus_rate >= 95:
        print("✓ STRONG CONSENSUS (>95%)")
        print("  - Most messages were consistently delivered")
        print("  - Minor inconsistencies may exist")
    else:
        print("⚠️ PARTIAL CONSENSUS")
        print(f"  - Only {consensus_rate:.1f}% of messages achieved full consensus")
        print("  - Review network connectivity and threshold settings")
    
    print(f"{'='*80}\n")

def main():
    """Main entry point."""
    if len(sys.argv) > 1:
        log_dir = Path(sys.argv[1])
    else:
        # Default to the logs directory
        log_dir = Path(__file__).parent / "logs"
    
    if not log_dir.exists():
        print(f"❌ Error: Log directory not found: {log_dir}")
        sys.exit(1)
    
    print(f"📁 Analyzing logs in: {log_dir}")
    
    node_data = load_node_deliveries(log_dir)
    
    if not node_data:
        print("❌ No node data could be loaded!")
        sys.exit(1)
    
    analyze_consensus(node_data)

if __name__ == "__main__":
    main()
