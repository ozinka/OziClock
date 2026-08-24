using System;
using System.Windows;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Shapes;

namespace Ozi.Utilities.Views
{
    public partial class ColorPicker : Window
    {
        public Color? SelectedColor { get; private set; }
        public Action<Color> OnColorSelected { get; set; }
        
        private bool _isClosing;

        public ColorPicker()
        {
            InitializeComponent();
            Deactivated += ColorPickerWindow_OnDeactivated;
        }

        private void ColorSelected(object sender, MouseButtonEventArgs e)
        {
            if (sender is Rectangle rectangle && rectangle.Fill is SolidColorBrush brush)
            {
                SelectedColor = brush.Color;
                OnColorSelected?.Invoke(brush.Color);
                _isClosing = true;
                Close();
            }
        }

        private void ColorPickerWindow_OnDeactivated(object? sender, EventArgs e)
        {
            if (_isClosing) return;

            _isClosing = true;
            Close();
        }
    }
}