import csv
import argparse
import requests

BASE_URL = "https://rest.uniprot.org/uniprotkb/search"


def read_accessions(csv_file, column_index=0, has_header=False):
    accessions = []

    with open(csv_file, newline="") as f:
        reader = csv.reader(f)

        if has_header:
            next(reader, None)

        for row in reader:
            if row and len(row) > column_index:
                acc = row[column_index].strip()
                if acc:
                    accessions.append(acc)

    return accessions


def read_queries(query_file):
    """Read query ranges from file (format: lower,upper per line)"""
    queries = []
    
    with open(query_file, newline="") as f:
        reader = csv.reader(f)
        for row in reader:
            if row and len(row) >= 2:
                try:
                    lower = int(row[0].strip())
                    upper = int(row[1].strip())
                    queries.append((lower, upper))
                except ValueError:
                    continue
    
    return queries


def write_query_file_with_header(query_file, output_prefix):
    """Write query file with header"""
    queries = read_queries(query_file)
    
    output_file = f"{output_prefix}_rust_queries.csv"
    
    with open(output_file, "w", newline="") as f:
        writer = csv.writer(f)
        writer.writerow(["lower", "upper"])
        for lower, upper in queries:
            writer.writerow([lower, upper])
    
    return queries


def write_limits_csv(accessions, num_queries, max_value, output_prefix):
    """Write limits CSV file"""
    output_file = f"{output_prefix}_limits.csv"
    
    with open(output_file, "w", newline="") as f:
        writer = csv.writer(f)
        
        # First row: #bins,<number_of_queries>
        writer.writerow(["#bins", num_queries])
        
        # Second row: bins,max,max,max,...
        row = ["bins"] + [max_value] * (num_queries+1)
        writer.writerow(row)
        
        # Subsequent rows: accession,max,max,max,...
        for accession in accessions:
            row = [accession] + [max_value] * (num_queries+1)
            writer.writerow(row)


def chunk_list(lst, size):
    for i in range(0, len(lst), size):
        yield lst[i:i + size]


def fetch_batch(accessions):
    query = "accession:(" + " OR ".join(accessions) + ")"

    params = {
        "query": query,
        "format": "txt"
    }

    response = requests.get(BASE_URL, params=params)

    if response.status_code != 200:
        raise RuntimeError(
            f"UniProt API error {response.status_code}: {response.text[:200]}"
        )

    return response.text


def main():
    parser = argparse.ArgumentParser(
        description="Fetch UniProt entries (TXT flat file) from a CSV of accessions"
    )

    parser.add_argument(
        "-i", "--input",
        required=True,
        help="Input CSV file containing accession IDs"
    )

    parser.add_argument(
        "-p", "--prefix",
        required=True,
        help="Output prefix for all generated files"
    )

    parser.add_argument(
        "-q", "--queries",
        required=True,
        help="Query file containing ranges (lower,upper per line)"
    )

    parser.add_argument(
        "-m", "--max",
        type=int,
        required=True,
        help="Max value for limits CSV"
    )

    parser.add_argument(
        "-c", "--column",
        type=int,
        default=0,
        help="CSV column index containing accession IDs (default: 0)"
    )

    parser.add_argument(
        "--header",
        action="store_true",
        help="Set this flag if CSV has a header row"
    )

    parser.add_argument(
        "--batch-size",
        type=int,
        default=100,
        help="Number of accessions per request (default: 100)"
    )

    args = parser.parse_args()

    # Read accessions
    accessions = read_accessions(args.input, args.column, args.header)

    if not accessions:
        raise SystemExit("No accessions found in input file")

    # Process queries and write query file with header
    queries = write_query_file_with_header(args.queries, args.prefix)
    # Write limits CSV
    write_limits_csv(accessions, len(queries), args.max, args.prefix)

    # Fetch UniProt entries
    uniprot_output = f"{args.prefix}_uniprot.txt"
    with open(uniprot_output, "w") as out:
        for i, batch in enumerate(chunk_list(accessions, args.batch_size), 1):
            print(f"Fetching batch {i} ({len(batch)} entries)")
            txt = fetch_batch(batch)
            out.write(txt)
            out.write("\n")


if __name__ == "__main__":
    main()
