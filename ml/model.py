import os
import torch
import torch.nn as nn
import torch.optim as optim
import numpy as np

torch.manual_seed(0)
np.random.seed(0)

# Define an LSTM model 
class LSTMModel(nn.Module):
    def __init__(self, input_size, hidden_size, num_layers, output_size):
        super(LSTMModel, self).__init__()
        self.lstm = nn.LSTM(input_size, hidden_size, num_layers, batch_first=True)
        self.fc = nn.Linear(hidden_size, output_size)
    
    def forward(self, x):
        # x shape: (batch_size, sequence_length, input_size)
        out, _ = self.lstm(x)
        # Use the output of the last time step
        out = out[:, -1, :]
        out = self.fc(out)
        return out

def create_synthetic_data(seq_length, num_samples):
    """
    Generate synthetic time-series data using a noisy sine wave.
    X: sequences of length `seq_length`
    y: the next value in the sequence
    """
    X = []
    y = []
    for _ in range(num_samples):
        start = np.random.rand() * 2 * np.pi
        xs = np.linspace(start, start + np.pi, seq_length + 1)
        data = np.sin(xs) + 0.1 * np.random.randn(seq_length + 1)
        X.append(data[:-1])
        y.append(data[-1])
    X = np.array(X)  # shape: (num_samples, seq_length)
    y = np.array(y)  # shape: (num_samples,)
    # Reshape X to (num_samples, seq_length, 1) and y to (num_samples, 1)
    X = X.reshape(-1, seq_length, 1)
    y = y.reshape(-1, 1)
    return torch.tensor(X, dtype=torch.float32), torch.tensor(y, dtype=torch.float32)

def train_model():
    # Hyperparameters
    input_size = 1
    hidden_size = 50
    num_layers = 2
    output_size = 1
    seq_length = 10
    num_samples = 1000
    num_epochs = 100
    learning_rate = 0.01

    # Generate synthetic training data
    X, y = create_synthetic_data(seq_length, num_samples)
    
    # Instantiate the model, loss function, and optimizer
    model = LSTMModel(input_size, hidden_size, num_layers, output_size)
    criterion = nn.MSELoss()
    optimizer = optim.Adam(model.parameters(), lr=learning_rate)

    # Training loop
    for epoch in range(num_epochs):
        model.train()
        optimizer.zero_grad()
        outputs = model(X)
        loss = criterion(outputs, y)
        loss.backward()
        optimizer.step()
        
        if (epoch + 1) % 10 == 0:
            print(f"Epoch [{epoch+1}/{num_epochs}], Loss: {loss.item():.4f}")

    # Save the model to the "model/" folder in the project root.
    os.makedirs("../model", exist_ok=True)
    model_path = "../model/lstm_model.pth"
    torch.save(model.state_dict(), model_path)
    print(f"Model saved to {model_path}")

if __name__ == "__main__":
    train_model()

