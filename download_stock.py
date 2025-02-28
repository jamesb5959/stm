import sys
import yfinance as yf
import os

if len(sys.argv) < 2:
    print("Usage: python download_stock.py <TICKER>")
    sys.exit(1)

ticker = sys.argv[1].upper()
data = yf.download(ticker, period="1y")  # Download 1 year of data
os.makedirs("pre_stock", exist_ok=True)
filename = os.path.join("pre_stock", f"{ticker}.csv")
data.to_csv(filename)
print(f"Downloaded data for {ticker} to {filename}")

