using System.Windows;
using System.Windows.Input;

namespace Ozi.Utilities;

public partial class FmEdit : Window
{
    public FmEdit()
    {
        InitializeComponent();
    }

    private void BtnColorPicker_Click(object sender, RoutedEventArgs e)
    {
        throw new System.NotImplementedException();
    }

    private void BtnOk_Click(object sender, RoutedEventArgs e)
    {
        Close();
    }

    private void BtnCancel_Click(object sender, RoutedEventArgs e)
    {
        Close();
    }

    private void Window_MouseDown(object sender, MouseButtonEventArgs e)
    {
        if (e.ChangedButton == MouseButton.Left)
        {
            DragMove();
        }
    }
}